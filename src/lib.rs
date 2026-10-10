//! Chakany Systems' offline PII redactor, available as Rust and C libraries.
pub mod engine;
pub mod ffi;
pub mod model;
pub mod policy;
pub mod tokenizer;

#[derive(Debug)]
pub enum RedactionError {
    Artifact,
    Runtime,
    Inference,
    Input,
}
impl std::fmt::Display for RedactionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Artifact => "invalid pinned artifacts",
            Self::Runtime => "native runtime unavailable",
            Self::Inference => "redaction inference failed",
            Self::Input => "unsupported diagnostic input",
        })
    }
}
impl std::error::Error for RedactionError {}
