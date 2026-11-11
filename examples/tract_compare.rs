//! Compare the original pinned graph's logits, spans, redaction and warm latency.
#[path = "../src/tract_weights.rs"]
mod tract_weights;
use defingerprinter::{
    engine::supported,
    model::{ARTIFACTS, Model},
    tokenizer::Tokenizer,
};
use ort::{
    session::{Session, builder::GraphOptimizationLevel},
    value::Tensor as OrtTensor,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{env, fs, hint::black_box, path::PathBuf, time::Instant};
use tract_onnx::prelude::*;
use tract_onnx::tract_core::internal::{ensure, format_err};

type TractSession = TypedSimpleState;

#[derive(Deserialize)]
struct Case {
    name: String,
    language: String,
    text: String,
}

fn cases() -> TractResult<Vec<Case>> {
    let mut cases: Vec<Case> = serde_json::from_str(include_str!("tract_cases.json"))?;
    for length in [3, 8, 32, 128, 256, 512] {
        cases.push(Case {
            name: format!("length-{length}"),
            language: "en".into(),
            text: "a ".repeat(length - 2),
        });
    }
    cases.push(Case {
        name: "multi-window".into(),
        language: "en".into(),
        text: format!(
            "{}Alice Smith called Bob Jones. {}",
            "a ".repeat(480),
            "a ".repeat(200)
        ),
    });
    Ok(cases)
}

fn ort_result<T, R>(result: Result<T, ort::Error<R>>) -> TractResult<T> {
    result.map_err(|error| format_err!("{error}"))
}

fn tract_logits(session: &mut TractSession, ids: &[i64]) -> TractResult<Vec<f32>> {
    let length = ids.len();
    let output = session.run(tvec![
        Tensor::from_shape(&[1, length], ids)?.into(),
        Tensor::from_shape(&[1, length], &vec![1i64; length])?.into(),
        Tensor::from_shape(&[1, length], &vec![0i64; length])?.into(),
    ])?;
    ensure!(
        output.len() == 1 && output[0].shape() == [1, length, 35],
        "unexpected Tract output shape"
    );
    Ok(output[0].try_as_plain_ram()?.as_slice::<f32>()?.to_vec())
}

fn ort_logits(session: &mut Session, ids: &[i64]) -> TractResult<Vec<f32>> {
    let length = ids.len();
    let output = session.run(ort::inputs![
        "input_ids" => OrtTensor::from_array(([1usize, length], ids.to_vec()))?,
        "attention_mask" => OrtTensor::from_array(([1usize, length], vec![1i64; length]))?,
        "token_type_ids" => OrtTensor::from_array(([1usize, length], vec![0i64; length]))?,
    ])?;
    let (shape, logits) = output["logits"].try_extract_tensor::<f32>()?;
    ensure!(
        shape.as_ref() == [1, length as i64, 35],
        "unexpected ORT output shape"
    );
    Ok(logits.to_vec())
}

fn argmax(row: &[f32]) -> usize {
    row.iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .unwrap()
        .0
}

fn probabilities(row: &[f32]) -> Vec<f32> {
    let max = row[argmax(row)];
    let values: Vec<_> = row.iter().map(|x| (x - max).exp()).collect();
    let sum: f32 = values.iter().sum();
    values.into_iter().map(|x| x / sum).collect()
}

fn median_ms(mut run: impl FnMut() -> TractResult<Vec<f32>>) -> TractResult<f64> {
    for _ in 0..3 {
        black_box(run()?);
    }
    let mut times = Vec::new();
    for _ in 0..15 {
        let start = Instant::now();
        black_box(run()?);
        times.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    times.sort_by(f64::total_cmp);
    Ok(times[times.len() / 2])
}

fn load_model(directory: &std::path::Path, expanded: bool) -> TractResult<Model> {
    Ok(if expanded {
        Model::load_tract_expanded(directory)?
    } else {
        Model::load_tract(directory)?
    })
}

fn main() -> TractResult<()> {
    let mut args: Vec<_> = env::args_os().skip(1).collect();
    let expanded = args.iter().any(|arg| arg == "--expanded");
    args.retain(|arg| arg != "--expanded");
    ensure!(
        (1..=2).contains(&args.len()),
        "usage: tract_compare MODEL_DIR [ONNX_RUNTIME_LIBRARY] [--expanded]"
    );
    let directory = PathBuf::from(&args[0]);
    println!(
        "Tract weight storage: {}",
        if expanded { "expanded" } else { "packed" }
    );
    let mut verified = Vec::new();
    for (name, hash) in ARTIFACTS {
        let bytes = fs::read(directory.join(name))?;
        ensure!(
            format!("{:x}", Sha256::digest(&bytes)) == hash,
            "pinned artifact hash mismatch: {name}"
        );
        verified.push(bytes);
    }
    let cases = cases()?;
    for case in &cases {
        ensure!(
            supported(&case.language, &case.text),
            "fixture rejected by admission: {}",
            case.name
        );
    }
    if args.len() == 1 {
        // This mode never initializes ORT or supplies a native runtime path.
        let mut model = load_model(&directory, expanded)?;
        for case in &cases {
            black_box(model.redact(&case.text)?);
        }
        println!(
            "Tract-only execution smoke: {} cases completed without ONNX Runtime",
            cases.len()
        );
        return Ok(());
    }
    let runtime = PathBuf::from(&args[1]);
    let tokenizer = Tokenizer::from_json(&verified[1])?;
    let start = Instant::now();
    let inference = tract_onnx::onnx().model_for_read(&mut std::io::Cursor::new(&verified[3]))?;
    let names: Vec<_> = inference
        .input_outlets()?
        .iter()
        .map(|outlet| inference.node(outlet.node).name.as_str())
        .collect();
    ensure!(
        names == ["input_ids", "attention_mask", "token_type_ids"],
        "unexpected Tract input order"
    );
    let mut typed = inference.into_typed()?;
    if expanded {
        let (packed, expanded) = tract_weights::expand_weights(&mut typed)?;
        println!("Expanded weights: packed_bytes={packed} expanded_bytes={expanded}");
    }
    let mut tract = typed.into_optimized()?.into_runnable()?.spawn()?;
    println!(
        "Tract load: {:.1} ms",
        start.elapsed().as_secs_f64() * 1000.0
    );
    let start = Instant::now();
    ort::init_from(&runtime)?.with_telemetry(false).commit();
    let builder = ort_result(Session::builder())?;
    let builder = ort_result(builder.with_intra_threads(1))?;
    let builder = ort_result(builder.with_inter_threads(1))?;
    let builder = ort_result(builder.with_intra_op_spinning(false))?;
    let builder = ort_result(builder.with_inter_op_spinning(false))?;
    let mut builder = ort_result(builder.with_optimization_level(GraphOptimizationLevel::Disable))?;
    let mut ort = ort_result(builder.commit_from_memory(&verified[3]))?;
    println!("ORT load: {:.1} ms", start.elapsed().as_secs_f64() * 1000.0);

    let mut failures = Vec::new();
    let mut max_logit_error = 0f32;
    let mut max_probability_error = 0f32;
    let mut tokens_checked = 0;
    let mut raw_cases = 0;
    for case in &cases {
        let mut ids = vec![2];
        ids.extend(tokenizer.tokenize(&case.text).iter().map(|t| t.id));
        ids.push(3);
        if ids.len() > 512 {
            continue;
        }
        raw_cases += 1;
        let actual = tract_logits(&mut tract, &ids)?;
        let reference = ort_logits(&mut ort, &ids)?;
        ensure!(
            actual.iter().chain(&reference).all(|x| x.is_finite()),
            "non-finite logits: {}",
            case.name
        );
        let error = actual
            .iter()
            .zip(&reference)
            .map(|(a, b)| (a - b).abs())
            .fold(0f32, f32::max);
        max_logit_error = max_logit_error.max(error);
        let mut mismatches = 0;
        let mut probability_error = 0f32;
        for (a, b) in actual
            .as_chunks::<35>()
            .0
            .iter()
            .zip(reference.as_chunks::<35>().0)
        {
            tokens_checked += 1;
            mismatches += usize::from(argmax(a) != argmax(b));
            probability_error = probability_error.max(
                probabilities(a)
                    .iter()
                    .zip(probabilities(b))
                    .map(|(a, b)| (a - b).abs())
                    .fold(0f32, f32::max),
            );
        }
        max_probability_error = max_probability_error.max(probability_error);
        if error > 0.02 || probability_error > 0.005 || mismatches != 0 {
            failures.push(format!(
                "{}: logits={error:.6} probabilities={probability_error:.6} labels={mismatches}",
                case.name
            ));
        }
    }
    println!(
        "Raw comparison: {raw_cases} cases, {tokens_checked} tokens, max_logit_error={max_logit_error:.6}, max_probability_error={max_probability_error:.6}"
    );
    println!(
        "Warm inference: 3 warmups, median of 15 calls, tensor construction/output copy included"
    );
    for length in [8, 32, 128, 256, 512, 8] {
        let mut ids = vec![tokenizer.tokenize("a")[0].id; length];
        ids[0] = 2;
        ids[length - 1] = 3;
        let tract_ms = median_ms(|| tract_logits(&mut tract, &ids))?;
        let ort_ms = median_ms(|| ort_logits(&mut ort, &ids))?;
        println!(
            "tokens={length} tract={tract_ms:.3}ms ort={ort_ms:.3}ms ratio={:.2}",
            tract_ms / ort_ms
        );
    }
    drop(tract);
    drop(ort);
    let mut tract = load_model(&directory, expanded)?;
    let mut ort = Model::load(&directory, &runtime)?;
    let mut span_count = 0;
    let mut max_score_error = 0f32;
    for case in &cases {
        let actual = tract.detect(&case.text)?;
        let reference = ort.detect(&case.text)?;
        span_count += reference.len();
        let same = actual.len() == reference.len()
            && actual
                .iter()
                .zip(&reference)
                .all(|(a, b)| a.start == b.start && a.end == b.end && a.label == b.label);
        if !same {
            failures.push(format!("{}: detected spans differ", case.name));
        }
        if same {
            for (a, b) in actual.iter().zip(&reference) {
                max_score_error = max_score_error.max((a.score - b.score).abs());
            }
        }
        let actual = tract.redact(&case.text)?;
        let reference = ort.redact(&case.text)?;
        if actual != reference {
            failures.push(format!("{}: redaction differs", case.name));
        }
    }
    if max_score_error > 0.005 {
        failures.push(format!("span score error={max_score_error:.6}"));
    }
    println!(
        "Redaction comparison: {} cases, {span_count} spans, max_score_error={max_score_error:.6}",
        cases.len()
    );
    for failure in &failures {
        eprintln!("FAIL: {failure}");
    }
    ensure!(failures.is_empty(), "{} parity failures", failures.len());
    println!("PASS: logits, token labels, detected spans and exact redaction output");
    Ok(())
}
