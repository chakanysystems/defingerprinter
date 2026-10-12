//! Actual-model parity. Generate reference fixtures and run via node ops/test-native-parity.mjs.
use defingerprinter::model::Model;
use serde_json::Value;
use std::{env, fs, path::PathBuf};

#[test]
#[ignore = "requires pinned model, local ORT and JS-generated fixtures; run ops/test-native-parity.mjs"]
fn pinned_reference_parity() {
    let required =
        |key: &str| {
            PathBuf::from(env::var(key).unwrap_or_else(|_| {
                panic!("{key} is required; run node ops/test-native-parity.mjs")
            }))
        };
    let fixture: Value =
        serde_json::from_slice(&fs::read(required("RAMPART_PARITY_FIXTURE")).unwrap()).unwrap();
    let mut model = Model::load(&required("RAMPART_MODEL_DIR"), &required("RAMPART_RUNTIME"))
        .expect("verified local model/runtime");
    let mut failures = Vec::new();
    for case in fixture["tokenizerCases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let text = case["text"].as_str().unwrap();
        let tokens = model.tokenizer.tokenize(text);
        let ids: Vec<i64> = tokens.iter().map(|t| t.id).collect();
        let expected: Vec<i64> = case["ids"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_i64().unwrap())
            .collect();
        let pieces: Vec<&str> = tokens.iter().map(|t| t.text.as_str()).collect();
        let expected_pieces: Vec<&str> = case["tokens"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        if ids != expected || pieces != expected_pieces {
            failures.push(format!(
                "tokenizer {name}: native={pieces:?} reference={expected_pieces:?}; ids match={}",
                ids == expected
            ));
        }
        for token in &tokens {
            assert!(
                token.start < token.end && token.end <= text.len(),
                "{name}: nonempty bounded token offset"
            );
            assert!(
                text.is_char_boundary(token.start) && text.is_char_boundary(token.end),
                "{name}: UTF-8 offset"
            );
        }
        assert!(
            tokens.windows(2).all(|t| t[0].end <= t[1].start),
            "{name}: monotone disjoint offsets"
        );
    }
    for case in fixture["heuristicCases"].as_array().unwrap() {
        let text = case["text"].as_str().unwrap();
        let mut observed: Vec<_> = defingerprinter::policy::deterministic(text)
            .into_iter()
            .map(|s| (s.start, s.end, s.label))
            .collect();
        let mut expected: Vec<_> = case["spans"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| {
                (
                    s["start"].as_u64().unwrap() as usize,
                    s["end"].as_u64().unwrap() as usize,
                    s["label"].as_str().unwrap().to_string(),
                )
            })
            .collect();
        observed.sort();
        expected.sort();
        if observed != expected {
            failures.push(format!(
                "deterministic {}: native={observed:?} reference={expected:?}",
                case["name"]
            ));
        }
    }
    for case in fixture["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let text = case["text"].as_str().unwrap();
        let spans = model.detect(text).unwrap();
        if name == "controls" {
            // The wire protocol rejects NUL/bell. Upstream reconstructs decoded words
            // through folded substring searches and drops groups across removed controls.
            // Native byte offsets deliberately retain those detections instead.
            assert!(spans.iter().all(|s| text.is_char_boundary(s.start)
                && text.is_char_boundary(s.end)
                && s.start < s.end
                && s.end <= text.len()));
            for person in ["Bob", "Smith"] {
                let start = text.find(person).unwrap();
                assert!(
                    spans
                        .iter()
                        .any(|s| s.start <= start && s.end >= start + person.len()),
                    "control cleanup lost {person}"
                );
            }
            println!(
                "Controls fixture: intentional direct-UTF8-offset divergence; protocol rejects this input"
            );
            continue;
        }
        let expected = case["spans"].as_array().unwrap();
        let same = spans.len() == expected.len()
            && spans.iter().zip(expected).all(|(s, e)| {
                s.start == e["start"].as_u64().unwrap() as usize
                    && s.end == e["end"].as_u64().unwrap() as usize
                    && s.label == e["label"].as_str().unwrap()
                    && (s.score as f64 - e["score"].as_f64().unwrap()).abs() < 0.02
            });
        if !same {
            let observed: Vec<_> = spans
                .iter()
                .map(|s| (s.start, s.end, &s.label, s.score, &text[s.start..s.end]))
                .collect();
            let reference: Vec<_> = expected
                .iter()
                .map(|s| {
                    let start = s["start"].as_u64().unwrap() as usize;
                    let end = s["end"].as_u64().unwrap() as usize;
                    (
                        start,
                        end,
                        s["label"].as_str().unwrap(),
                        s["score"].as_f64().unwrap(),
                        &text[start..end],
                    )
                })
                .collect();
            failures.push(format!(
                "NER {name}: native={observed:?} reference={reference:?}"
            ));
        }
    }
    let placeholder = regex::Regex::new(r"\[([A-Z_]+)_\d+\]").unwrap();
    for case in fixture["redactCases"].as_array().unwrap() {
        let text = case["text"].as_str().unwrap();
        let safe = model.redact(text).unwrap();
        let normalized = placeholder.replace_all(&safe, "[$1]");
        if normalized != case["expected"].as_str().unwrap() {
            failures.push(format!(
                "redact {}: native={normalized} reference={}",
                case["name"], case["expected"]
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    println!(
        "Pinned parity: {} tokenizer, {} heuristic, {} NER (5 long seams, 1 explicit rejected-controls divergence), {} structured redact cases passed",
        fixture["tokenizerCases"].as_array().unwrap().len(),
        fixture["heuristicCases"].as_array().unwrap().len(),
        fixture["cases"].as_array().unwrap().len(),
        fixture["redactCases"].as_array().unwrap().len()
    );
}
