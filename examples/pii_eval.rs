//! Measure span precision and recall on an annotated JSONL corpus.
//!
//! Corpus offsets count characters. Model spans count UTF-8 bytes. Only labels
//! with a one-to-one mapping take part in scoring.
use defingerprinter::model::Model;
use serde_json::Value;
use std::{collections::BTreeMap, env, fs, path::PathBuf};

const LABELS: [(&str, &str); 17] = [
    ("GIVENNAME1", "GIVEN_NAME"),
    ("GIVENNAME2", "GIVEN_NAME"),
    ("GIVENNAME3", "GIVEN_NAME"),
    ("LASTNAME1", "SURNAME"),
    ("LASTNAME2", "SURNAME"),
    ("LASTNAME3", "SURNAME"),
    ("EMAIL", "EMAIL"),
    ("TEL", "PHONE"),
    ("BUILDING", "BUILDING_NUMBER"),
    ("STREET", "STREET_NAME"),
    ("SECADDRESS", "SECONDARY_ADDRESS"),
    ("CITY", "CITY"),
    ("STATE", "STATE"),
    ("POSTCODE", "ZIP_CODE"),
    ("PASSPORT", "PASSPORT"),
    ("DRIVERLICENSE", "DRIVERS_LICENSE"),
    ("IDCARD", "GOVERNMENT_ID"),
];

fn model_label(corpus: &str) -> Option<&'static str> {
    LABELS
        .iter()
        .find(|(name, _)| *name == corpus)
        .map(|(_, label)| *label)
}

fn scored_label(label: &str) -> Option<&'static str> {
    LABELS
        .iter()
        .find(|(_, name)| *name == label)
        .map(|(_, name)| *name)
}

fn ratio(hits: usize, total: usize) -> String {
    if total == 0 {
        "n/a".into()
    } else {
        format!("{:.3}", hits as f64 / total as f64)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    let directory = PathBuf::from(
        args.first()
            .ok_or("usage: MODEL_DIR CORPUS [RECORDS] [RUNTIME]")?,
    );
    let corpus = PathBuf::from(args.get(1).ok_or("corpus path required")?);
    let limit: usize = args
        .get(2)
        .map(|value| value.parse())
        .transpose()?
        .unwrap_or(500);
    let mut model = Model::load_tract_expanded(&directory)?;
    let mut reference = args
        .get(3)
        .map(|path| Model::load(&directory, &PathBuf::from(path)))
        .transpose()?;
    let text = fs::read_to_string(corpus)?;
    let mut tally: BTreeMap<&'static str, [usize; 3]> = BTreeMap::new();
    let mut unscored: BTreeMap<String, usize> = BTreeMap::new();
    let mut bad_offsets = 0usize;
    let mut records = 0usize;
    let mut agree = 0usize;
    let mut compared = 0usize;
    let mut reported = 0usize;
    for line in text.lines().take(limit) {
        let record: Value = serde_json::from_str(line)?;
        let source = record["source_text"]
            .as_str()
            .ok_or("source_text missing")?;
        let mut byte_at = Vec::with_capacity(source.chars().count() + 1);
        for (offset, _) in source.char_indices() {
            byte_at.push(offset);
        }
        byte_at.push(source.len());
        let mut gold = Vec::new();
        for mask in record["privacy_mask"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or(&[])
        {
            let Some(label) = mask["label"].as_str().and_then(model_label) else {
                continue;
            };
            let (Some(start), Some(end)) = (mask["start"].as_u64(), mask["end"].as_u64()) else {
                continue;
            };
            let (start, end) = (start as usize, end as usize);
            if start >= end || end >= byte_at.len() {
                bad_offsets += 1;
                continue;
            }
            if let Some(value) = mask["value"].as_str() {
                let actual: String = source.chars().skip(start).take(end - start).collect();
                if actual != value {
                    bad_offsets += 1;
                    continue;
                }
            }
            gold.push((byte_at[start], byte_at[end], label));
        }
        records += 1;
        let predicted = model.detect(source)?;
        if let Some(reference) = reference.as_mut() {
            let mut left: Vec<_> = predicted
                .iter()
                .map(|span| (span.start, span.end, span.label.clone()))
                .collect();
            let mut right: Vec<_> = reference
                .detect(source)?
                .iter()
                .map(|span| (span.start, span.end, span.label.clone()))
                .collect();
            left.sort();
            right.sort();
            compared += 1;
            if left == right {
                agree += 1;
            } else if reported < 5 {
                reported += 1;
                let only_left: Vec<_> = left.iter().filter(|span| !right.contains(span)).collect();
                let only_right: Vec<_> = right.iter().filter(|span| !left.contains(span)).collect();
                println!(
                    "mismatch record {records}: tract_only {only_left:?} ort_only {only_right:?}"
                );
            }
        }
        let mut used = vec![false; predicted.len()];
        for &(start, end, label) in &gold {
            let hit = predicted.iter().enumerate().position(|(index, span)| {
                !used[index] && span.label == label && span.start == start && span.end == end
            });
            let counts = tally.entry(label).or_insert([0; 3]);
            match hit {
                Some(index) => {
                    used[index] = true;
                    counts[0] += 1;
                }
                None => counts[2] += 1,
            }
        }
        for (index, span) in predicted.iter().enumerate() {
            if used[index] {
                continue;
            }
            match scored_label(&span.label) {
                Some(label) => tally.entry(label).or_insert([0; 3])[1] += 1,
                None => *unscored.entry(span.label.clone()).or_insert(0) += 1,
            }
        }
    }
    if compared > 0 {
        println!("backend agreement {agree}/{compared} records");
    }
    let mut totals = [0usize; 3];
    println!("records {records} bad_offsets {bad_offsets} unscored {unscored:?}");
    println!(
        "{:<20} {:>6} {:>6} {:>6} {:>10} {:>10}",
        "label", "tp", "fp", "fn", "precision", "recall"
    );
    for (label, [tp, fp, missing]) in &tally {
        for (total, count) in totals.iter_mut().zip([tp, fp, missing]) {
            *total += count;
        }
        println!(
            "{label:<20} {tp:>6} {fp:>6} {missing:>6} {:>10} {:>10}",
            ratio(*tp, tp + fp),
            ratio(*tp, tp + missing)
        );
    }
    println!(
        "{:<20} {:>6} {:>6} {:>6} {:>10} {:>10}",
        "all",
        totals[0],
        totals[1],
        totals[2],
        ratio(totals[0], totals[0] + totals[1]),
        ratio(totals[0], totals[0] + totals[2])
    );
    Ok(())
}
