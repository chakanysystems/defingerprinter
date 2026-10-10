//! Pinned BERT WordPiece tokenization, with UTF-8 byte offsets and no general tokenizer runtime.
use crate::RedactionError;
use serde::Deserialize;
use std::collections::HashMap;
use unicode_general_category::{GeneralCategory as G, get_general_category};
use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};

#[derive(Clone, Debug)]
pub struct Token {
    pub id: i64,
    pub text: String,
    pub start: usize,
    pub end: usize,
}
pub struct Tokenizer {
    vocab: HashMap<String, i64>,
    max_word: usize,
}
#[derive(Deserialize)]
struct Definition {
    model: Model,
    normalizer: serde_json::Value,
    pre_tokenizer: serde_json::Value,
}
#[derive(Deserialize)]
struct Model {
    #[serde(rename = "type")]
    kind: String,
    vocab: HashMap<String, i64>,
    max_input_chars_per_word: usize,
    unk_token: String,
    continuing_subword_prefix: String,
}
impl Tokenizer {
    pub fn from_json(bytes: &[u8]) -> Result<Self, RedactionError> {
        let d: Definition = serde_json::from_slice(bytes).map_err(|_| RedactionError::Artifact)?;
        if d.model.kind != "WordPiece"
            || d.model.unk_token != "[UNK]"
            || d.model.continuing_subword_prefix != "##"
            || d.normalizer["type"] != "BertNormalizer"
            || d.normalizer["lowercase"] != true
            || d.normalizer["clean_text"] != true
            || d.pre_tokenizer["type"] != "BertPreTokenizer"
            || d.model.vocab.len() != 19730
            || d.model.vocab.get("[CLS]") != Some(&2)
            || d.model.vocab.get("[SEP]") != Some(&3)
        {
            return Err(RedactionError::Artifact);
        }
        Ok(Self {
            vocab: d.model.vocab,
            max_word: d.model.max_input_chars_per_word,
        })
    }
    pub fn tokenize(&self, raw: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut cursor = 0;
        while cursor < raw.len() {
            // Added special tokens are recognized before lowercase and punctuation splitting.
            if let Some(special) = ["[PAD]", "[UNK]", "[CLS]", "[SEP]", "[MASK]"]
                .into_iter()
                .find(|s| raw[cursor..].starts_with(s))
            {
                tokens.push(Token {
                    id: self.vocab[special],
                    text: special.into(),
                    start: cursor,
                    end: cursor + special.len(),
                });
                cursor += special.len();
                continue;
            }
            let first = raw[cursor..].chars().next().unwrap();
            if first.is_whitespace() || control(first) {
                cursor += first.len_utf8();
                continue;
            }
            let mut end = cursor + first.len_utf8();
            if !punctuation(first) && !cjk(first) {
                while end < raw.len() {
                    let c = raw[end..].chars().next().unwrap();
                    if c.is_whitespace()
                        || punctuation(c)
                        || cjk(c)
                        || ["[PAD]", "[UNK]", "[CLS]", "[SEP]", "[MASK]"]
                            .into_iter()
                            .any(|s| raw[end..].starts_with(s))
                    {
                        break;
                    }
                    end += c.len_utf8();
                }
            }
            let mut chars = Vec::new();
            for (offset, c) in raw[cursor..end].char_indices() {
                if control(c) {
                    continue;
                }
                // BERT strips accents with canonical decomposition; compatibility folding is only for offset recovery upstream.
                for lower in c.to_lowercase() {
                    for normalized in lower.to_string().nfd().filter(|c| !is_combining_mark(*c)) {
                        chars.push((normalized, cursor + offset, cursor + offset + c.len_utf8()));
                    }
                }
            }
            if chars.len() > self.max_word {
                tokens.push(Token {
                    id: 1,
                    text: "[UNK]".into(),
                    start: cursor,
                    end,
                });
            } else if !chars.is_empty() {
                let mut pieces = Vec::new();
                let mut start = 0;
                while start < chars.len() {
                    let found = (start + 1..=chars.len()).rev().find_map(|stop| {
                        let mut piece = if start == 0 {
                            String::new()
                        } else {
                            "##".into()
                        };
                        piece.extend(chars[start..stop].iter().map(|c| c.0));
                        self.vocab.get(&piece).map(|id| (stop, piece, *id))
                    });
                    let Some((stop, text, id)) = found else {
                        pieces.clear();
                        pieces.push(Token {
                            id: 1,
                            text: "[UNK]".into(),
                            start: cursor,
                            end,
                        });
                        break;
                    };
                    pieces.push(Token {
                        id,
                        text,
                        start: chars[start].1,
                        end: chars[stop - 1].2,
                    });
                    start = stop;
                }
                tokens.extend(pieces);
            }
            cursor = end;
        }
        tokens
    }
}
fn control(c: char) -> bool {
    c == '\0'
        || c == '\u{fffd}'
        || matches!(get_general_category(c), G::Control | G::Format)
            && !matches!(c, '\t' | '\n' | '\r')
}
fn punctuation(c: char) -> bool {
    let n = c as u32;
    (33..=47).contains(&n)
        || (58..=64).contains(&n)
        || (91..=96).contains(&n)
        || (123..=126).contains(&n)
        || matches!(
            get_general_category(c),
            G::ConnectorPunctuation
                | G::DashPunctuation
                | G::ClosePunctuation
                | G::FinalPunctuation
                | G::InitialPunctuation
                | G::OpenPunctuation
                | G::OtherPunctuation
        )
}
fn cjk(c: char) -> bool {
    let n = c as u32;
    [
        (0x4e00, 0x9fff),
        (0x3400, 0x4dbf),
        (0x20000, 0x2a6df),
        (0x2a700, 0x2b73f),
        (0x2b740, 0x2b81f),
        (0x2b820, 0x2ceaf),
        (0xf900, 0xfaff),
        (0x2f800, 0x2fa1f),
    ]
    .iter()
    .any(|(a, b)| (*a..=*b).contains(&n))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tokenizer() -> Tokenizer {
        let mut vocab = HashMap::new();
        for (i, w) in [
            "[PAD]", "[UNK]", "[CLS]", "[SEP]", "[MASK]", "jose", "##s", ".", "rivera",
        ]
        .iter()
        .enumerate()
        {
            vocab.insert(w.to_string(), i as i64);
        }
        Tokenizer {
            vocab,
            max_word: 100,
        }
    }
    #[test]
    fn accent_offsets_and_wordpieces() {
        let t = tokenizer().tokenize("JoséS Rivera.");
        assert_eq!(t.iter().map(|x| x.id).collect::<Vec<_>>(), vec![5, 6, 8, 7]);
        assert_eq!(
            (&"JoséS Rivera."[t[0].start..t[0].end], t[0].end),
            ("José", 5)
        );
    }
    #[test]
    fn unknown_word_and_special_tokens() {
        let t = tokenizer().tokenize("[MASK] unknown");
        assert_eq!(t.iter().map(|x| x.id).collect::<Vec<_>>(), vec![4, 1]);
        assert_eq!((t[1].start, t[1].end), (7, 14));
    }
}
