//! Verified local artifacts and single-threaded CPU token classification.
use crate::{
    RedactionError,
    policy::{self, Span},
    tokenizer::{Token, Tokenizer},
};
#[cfg(feature = "ort")]
use ort::{
    session::{Session, builder::GraphOptimizationLevel},
    value::Tensor,
};
use sha2::{Digest, Sha256};
#[cfg(feature = "ort")]
use std::sync::Arc;
use std::{fs, path::Path};

pub const ARTIFACTS: [(&str, &str); 4] = [
    (
        "config.json",
        "003b84bbcd489f5e782fe5cad8f3249c3653ec880089abb1ccc398a0d895e3e6",
    ),
    (
        "tokenizer.json",
        "98ade711428b42a1b5343c403a73344535e92de8e19359cdb567ef34da210259",
    ),
    (
        "tokenizer_config.json",
        "0088a6f8bcdd4014184fb068b83ebb12896a9db2bb269a71f73de83fef24bceb",
    ),
    (
        "onnx/model_q4.onnx",
        "9f27d24949b0581701071ea5ef522d77ccd3f50c525cc91eac4d265b0fc2afe5",
    ),
];
pub struct Model {
    session: Backend,
    pub tokenizer: Tokenizer,
    labels: Vec<String>,
}
impl Model {
    #[cfg(feature = "ort")]
    pub fn load(directory: &Path, runtime: &Path) -> Result<Self, RedactionError> {
        let VerifiedArtifacts {
            bytes,
            tokenizer,
            labels,
        } = VerifiedArtifacts::load(directory)?;
        ort::init_from(runtime)
            .map_err(|_| RedactionError::Runtime)?
            .with_name("defingerprinter")
            .with_telemetry(false)
            .with_logger(Arc::new(|_, _, _, _, _| {}))
            .commit();
        let session = Session::builder()
            .map_err(|_| RedactionError::Runtime)?
            .with_intra_threads(1)
            .map_err(|_| RedactionError::Runtime)?
            .with_inter_threads(1)
            .map_err(|_| RedactionError::Runtime)?
            .with_intra_op_spinning(false)
            .map_err(|_| RedactionError::Runtime)?
            .with_inter_op_spinning(false)
            .map_err(|_| RedactionError::Runtime)?
            // The reduced runtime contains only the pinned graph's operators, not optimizer-created fusions.
            .with_optimization_level(GraphOptimizationLevel::Disable)
            .map_err(|_| RedactionError::Runtime)?
            .commit_from_memory(&bytes[3])
            .map_err(|_| RedactionError::Runtime)?;
        Ok(Self {
            session: Backend::Ort(session),
            tokenizer,
            labels,
        })
    }
    /// Load the pinned graph with the experimental Rust-only Tract CPU backend.
    #[cfg(feature = "tract")]
    pub fn load_tract(directory: &Path) -> Result<Self, RedactionError> {
        Self::load_tract_weights(directory, false)
    }
    /// Load Tract with the Q4 weight values expanded to float32 once at load time.
    /// This permits faster matrix kernels but uses more memory. Model files and
    /// represented weight values are unchanged.
    #[cfg(feature = "tract")]
    pub fn load_tract_expanded(directory: &Path) -> Result<Self, RedactionError> {
        Self::load_tract_weights(directory, true)
    }
    #[cfg(feature = "tract")]
    fn load_tract_weights(directory: &Path, expanded: bool) -> Result<Self, RedactionError> {
        use tract_onnx::prelude::*;
        let VerifiedArtifacts {
            bytes,
            tokenizer,
            labels,
        } = VerifiedArtifacts::load(directory)?;
        let session = tract_onnx::onnx()
            .model_for_read(&mut std::io::Cursor::new(&bytes[3]))
            .and_then(|model| model.into_typed())
            .and_then(|mut model| {
                if expanded {
                    crate::tract_weights::expand_weights(&mut model)?;
                }
                model.into_optimized()
            })
            .and_then(|model| model.into_runnable())
            .and_then(|plan| plan.spawn())
            .map_err(|_| RedactionError::Runtime)?;
        Ok(Self {
            session: Backend::Tract(Box::new(session)),
            tokenizer,
            labels,
        })
    }
    pub fn redact(&mut self, raw: &str) -> Result<String, RedactionError> {
        let raw = policy::pre_redact(raw);
        let deterministic = policy::deterministic(&raw);
        let masked = policy::premask_mapped(&raw, &deterministic);
        let mut spans = deterministic;
        for span in self.detect(&masked.text)? {
            if let Some(span) = policy::project_masked_span(&masked, &span) {
                spans.push(span);
            }
        }
        let spans = policy::merge_spans(&raw, spans);
        Ok(policy::pre_redact(&policy::replace(&raw, &spans)))
    }
    pub fn detect(&mut self, raw: &str) -> Result<Vec<Span>, RedactionError> {
        let windows = plan_windows(raw, &self.tokenizer);
        let mut spans = Vec::new();
        for (start, end) in windows {
            let text = raw[start..end].replace('-', " ");
            let tokens = self.tokenizer.tokenize(&text);
            if tokens.len() > 510 {
                return Err(RedactionError::Input);
            }
            if tokens.is_empty() {
                continue;
            }
            let candidates = self.classify(&tokens)?;
            for mut span in policy::repair_ner(&text, candidates, 0.4) {
                span.start += start;
                span.end += start;
                spans.push(span);
            }
        }
        Ok(policy::merge_spans(raw, spans))
    }
    fn classify(&mut self, tokens: &[Token]) -> Result<Vec<Span>, RedactionError> {
        let mut ids = vec![2];
        ids.extend(tokens.iter().map(|t| t.id));
        ids.push(3);
        let logits = self.session.logits(ids)?;
        let mut groups: Vec<(Span, usize)> = Vec::new();
        for (index, token) in tokens.iter().enumerate() {
            // The pinned Transformers classifier excludes all five added special-token IDs.
            if token.id <= 4 {
                continue;
            }
            let row = &logits[(index + 1) * 35..(index + 2) * 35];
            let (label, max) = row
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .unwrap();
            let score = 1.0 / row.iter().map(|v| (v - max).exp()).sum::<f32>();
            let label = &self.labels[label];
            if label == "O" {
                continue;
            }
            let (prefix, kind) = label.split_once('-').ok_or(RedactionError::Artifact)?;
            if let Some((span, count)) = groups.last_mut()
                && span.label == kind
                && prefix != "B"
            {
                span.end = token.end;
                span.score += score;
                *count += 1;
                continue;
            }
            groups.push((
                Span {
                    start: token.start,
                    end: token.end,
                    label: kind.into(),
                    score,
                },
                1,
            ));
        }
        // Transformers 4.3.1's SIMPLE strategy emits group labels without offsets or BIO prefixes.
        // Rampart subsequently merges consecutive same-label groups and averages their means.
        let mut aggregated: Vec<(Span, usize)> = Vec::new();
        for (mut span, count) in groups {
            span.score /= count as f32;
            if let Some((previous, groups)) = aggregated.last_mut()
                && previous.label == span.label
            {
                previous.end = span.end;
                previous.score += span.score;
                *groups += 1;
            } else {
                aggregated.push((span, 1));
            }
        }
        Ok(aggregated
            .into_iter()
            .filter_map(|(mut span, count)| {
                span.score /= count as f32;
                (span.score >= 0.15).then_some(span)
            })
            .collect())
    }
}

// Both backends share artifact verification, tokenization and span policy.
struct VerifiedArtifacts {
    bytes: Vec<Vec<u8>>,
    tokenizer: Tokenizer,
    labels: Vec<String>,
}
impl VerifiedArtifacts {
    fn load(directory: &Path) -> Result<Self, RedactionError> {
        let mut bytes = Vec::new();
        for (name, hash) in ARTIFACTS {
            let path = directory.join(name);
            if fs::metadata(&path)
                .map_err(|_| RedactionError::Artifact)?
                .len()
                > 20_000_000
            {
                return Err(RedactionError::Artifact);
            }
            let data = fs::read(path).map_err(|_| RedactionError::Artifact)?;
            if format!("{:x}", Sha256::digest(&data)) != hash {
                return Err(RedactionError::Artifact);
            }
            bytes.push(data);
        }
        let config: serde_json::Value =
            serde_json::from_slice(&bytes[0]).map_err(|_| RedactionError::Artifact)?;
        let labels = (0..35)
            .map(|i| {
                config["id2label"][i.to_string()]
                    .as_str()
                    .map(String::from)
                    .ok_or(RedactionError::Artifact)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let tokenizer = Tokenizer::from_json(&bytes[1])?;
        Ok(Self {
            bytes,
            tokenizer,
            labels,
        })
    }
}

enum Backend {
    #[cfg(feature = "ort")]
    Ort(Session),
    #[cfg(feature = "tract")]
    Tract(Box<tract_onnx::prelude::TypedSimpleState>),
}
impl Backend {
    fn logits(&mut self, ids: Vec<i64>) -> Result<Vec<f32>, RedactionError> {
        let length = ids.len();
        let logits = match self {
            #[cfg(feature = "ort")]
            Self::Ort(session) => {
                let input = Tensor::from_array(([1usize, length], ids))
                    .map_err(|_| RedactionError::Inference)?;
                let mask = Tensor::from_array(([1usize, length], vec![1i64; length]))
                    .map_err(|_| RedactionError::Inference)?;
                let types = Tensor::from_array(([1usize, length], vec![0i64; length]))
                    .map_err(|_| RedactionError::Inference)?;
                let output = session.run(ort::inputs!["input_ids"=>input,"attention_mask"=>mask,"token_type_ids"=>types]).map_err(|_| RedactionError::Inference)?;
                let value = output.get("logits").ok_or(RedactionError::Inference)?;
                let (shape, logits) = value
                    .try_extract_tensor::<f32>()
                    .map_err(|_| RedactionError::Inference)?;
                if shape.as_ref() != [1, length as i64, 35] {
                    return Err(RedactionError::Inference);
                }
                logits.to_vec()
            }
            #[cfg(feature = "tract")]
            Self::Tract(session) => {
                use tract_onnx::prelude::{Tensor, tvec};
                let tensor = |data: &[i64]| {
                    Tensor::from_shape(&[1, length], data).map_err(|_| RedactionError::Inference)
                };
                let output = session
                    .run(tvec![
                        tensor(&ids)?.into(),
                        tensor(&vec![1; length])?.into(),
                        tensor(&vec![0; length])?.into()
                    ])
                    .map_err(|_| RedactionError::Inference)?;
                let value = output.first().ok_or(RedactionError::Inference)?;
                if output.len() != 1 || value.shape() != [1, length, 35] {
                    return Err(RedactionError::Inference);
                }
                value
                    .try_as_plain_ram()
                    .and_then(|view| Ok(view.as_slice::<f32>()?.to_vec()))
                    .map_err(|_| RedactionError::Inference)?
            }
        };
        if logits.iter().any(|v| !v.is_finite()) {
            return Err(RedactionError::Inference);
        }
        Ok(logits)
    }
}

// The pinned classifier packs words plus trailing whitespace, with a 500-token budget and >=64 overlap.
fn plan_windows(raw: &str, tokenizer: &Tokenizer) -> Vec<(usize, usize)> {
    let mut segments = Vec::new();
    let mut start = 0;
    while start < raw.len() {
        let mut end = start;
        while end < raw.len() && !raw[end..].chars().next().unwrap().is_whitespace() {
            end += raw[end..].chars().next().unwrap().len_utf8();
        }
        while end < raw.len() && raw[end..].chars().next().unwrap().is_whitespace() {
            end += raw[end..].chars().next().unwrap().len_utf8();
        }
        let mut from = start;
        while from < end {
            let count = tokenizer.tokenize(&raw[from..end]).len();
            if count <= 500 {
                segments.push((from, end, count));
                break;
            }
            let boundaries: Vec<_> = raw[from..end]
                .char_indices()
                .skip(1)
                .map(|(i, _)| from + i)
                .chain(std::iter::once(end))
                .collect();
            let mut lo = 0;
            let mut hi = boundaries.len();
            let mut best = boundaries[0];
            while lo < hi {
                let mid = (lo + hi) / 2;
                if tokenizer.tokenize(&raw[from..boundaries[mid]]).len() <= 500 {
                    best = boundaries[mid];
                    lo = mid + 1;
                } else {
                    hi = mid;
                }
            }
            segments.push((from, best, tokenizer.tokenize(&raw[from..best]).len()));
            from = best;
        }
        start = end;
    }
    let mut windows = Vec::new();
    let mut i = 0;
    while i < segments.len() {
        let mut total = 0;
        let mut j = i;
        while j < segments.len() && (j == i || total + segments[j].2 <= 500) {
            total += segments[j].2;
            j += 1;
        }
        windows.push((segments[i].0, segments[j - 1].1));
        if j == segments.len() {
            break;
        }
        let mut next = j;
        let mut shared = 0;
        while next > i + 1 && shared < 64 {
            next -= 1;
            shared += segments[next].2;
        }
        i = next;
    }
    windows
}
