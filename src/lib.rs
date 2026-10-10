//! Offline PII redaction for diagnostic text.
//!
//! Use [`engine::Redactor`] from Rust, or the version-2 [`ffi`] API from C,
//! C++, Python, Common Lisp, and other languages with a C FFI. The C declarations
//! are in `include/defingerprinter.h`; both interfaces use the same engine.
//!
//! # Deployment and lifecycle
//!
//! Construction does not read artifacts or load ONNX Runtime. Call
//! [`engine::Redactor::protect`] directly: the first supported request verifies
//! the pinned model files and starts a local CPU session. Subsequent requests
//! reuse that session. Dropping the redactor releases the model session.
//! The application owns collection, retention, and privacy policy.
//!
//! Supply the pinned model directory and the ONNX Runtime shared-library path,
//! normally as absolute paths. Nix builds can embed both through
//! [`engine::compiled_paths`]. No model download is performed by this library.
//! ONNX Runtime initialization is process-global: use one compatible runtime
//! for the entire process, including other users of the `ort` crate. Dropping
//! a model session does not unload or reset that process-global runtime.
//!
//! # Rust quick start
//!
//! This example needs the pinned artifacts and a compatible native runtime.
//!
//! ```no_run
//! use defingerprinter::engine::Redactor;
//! use std::path::PathBuf;
//!
//! let mut redactor = Redactor::new(
//!     PathBuf::from("/absolute/path/to/model"),
//!     PathBuf::from("/absolute/path/to/libonnxruntime.so"),
//! );
//! let result = redactor.protect("en", "Synthetic contact: alice@example.com");
//! match result {
//!     Ok(sanitized) => println!("{sanitized}"),
//!     Err(error) => eprintln!("Diagnostic withheld: {error}"),
//! }
//! drop(redactor);
//! ```
//!
//! # Admission and failure handling
//!
//! Supported language tags are `en`, `es`, `fr`, `de`, `it`, `pt`, and `nl`.
//! Input limits and Unicode admission rules are documented by
//! [`engine::supported`]. The language tag is an admission check, not a
//! language detector. Successful output is limited to 1,024 Unicode scalar
//! values and may be truncated.
//!
//! On error, withhold the diagnostic rather than forwarding the original or
//! treating an error as successful redaction. Redaction combines deterministic
//! rules and model inference; it is not a guarantee that every identifying
//! detail has been removed. The low-level [`model`], [`policy`], and
//! [`tokenizer`] modules expose building blocks; use [`engine::Redactor`] to
//! apply input admission and local redaction.
pub mod engine;
pub mod ffi;
pub mod model;
pub mod policy;
pub mod tokenizer;

/// Failure category returned by the Rust redaction pipeline.
///
/// Errors carry no diagnostic text. [`engine::Redactor::protect`] drops its
/// cached session after a loading or model-processing failure, so a later
/// supported request can retry loading.
#[derive(Debug)]
pub enum RedactionError {
    /// A pinned model file is missing, oversized, malformed, or has the wrong hash.
    Artifact,
    /// The native runtime could not be loaded or a model session could not be created.
    Runtime,
    /// Tensor construction, model execution, or model output validation failed.
    Inference,
    /// Input violates admission policy or token limits are exceeded.
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
