//! Guard the Tract backend against upstream changes.
//!
//! The expanded loader requires 37 Q4 weight constants in the pinned graph.
//! A Tract upgrade that changes constant or block-quant storage handling fails
//! this test before it can fail at request time.
#![cfg(feature = "tract")]

use defingerprinter::model::Model;
use std::{env, path::PathBuf};

const TEXT: &str = "My name is Walter Hartwell White. I live at 308 Negra Arroyo Lane, Albuquerque, New Mexico, 87104.";

#[test]
#[ignore = "requires pinned model; set RAMPART_MODEL_DIR"]
fn expanded_weights_match_packed_weights() {
    let directory =
        PathBuf::from(env::var("RAMPART_MODEL_DIR").expect("RAMPART_MODEL_DIR required"));
    let mut packed = Model::load_tract(&directory).expect("packed Tract loader");
    let mut expanded = Model::load_tract_expanded(&directory).expect("expanded Tract loader");
    let packed_text = packed.redact(TEXT).expect("packed redaction");
    let expanded_text = expanded.redact(TEXT).expect("expanded redaction");
    assert_eq!(packed_text, expanded_text);
    assert!(!packed_text.contains("Walter"));
    assert!(!packed_text.contains("Negra Arroyo"));
}
