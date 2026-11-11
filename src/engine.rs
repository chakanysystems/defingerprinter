//! Shared input policy and lazily loaded local model session.
use crate::{RedactionError, model::Model};
use std::path::{Path, PathBuf};

/// Maximum accepted UTF-8 input size in bytes (16,384).
///
/// [`MAX_INPUT_CHARS`] applies independently.
pub const MAX_INPUT_BYTES: usize = 16_384;
/// Maximum accepted input length in Unicode scalar values (4,096).
pub const MAX_INPUT_CHARS: usize = 4096;
/// Maximum returned length in Unicode scalar values (1,024).
pub const MAX_OUTPUT_CHARS: usize = 1024;
/// Required C ABI output buffer capacity in bytes (4,096).
///
/// Four bytes per output scalar value cover any UTF-8 result. The C ABI does
/// not append a NUL terminator.
pub const OUTPUT_CAPACITY: usize = MAX_OUTPUT_CHARS * 4;

/// A lazily loaded local redaction session.
///
/// Use a mutable instance to serialize calls. Dropping it drops its model
/// session, but does not unload the process-global native runtime. Input text is
/// not retained for retries. The caller owns collection and privacy policy.
///
/// ```
/// use defingerprinter::{engine::Redactor, RedactionError};
/// let mut redactor = Redactor::new("/missing/model".into(), "/missing/runtime".into());
/// assert!(matches!(redactor.redact("EN", "Synthetic text"), Err(RedactionError::Input)));
/// assert!(matches!(redactor.redact("en", "Synthetic text"), Err(RedactionError::Artifact)));
/// ```
pub struct Redactor {
    model: Option<Model>,
    directory: PathBuf,
    runtime: PathBuf,
}
impl Redactor {
    /// Construct a redactor with a model directory and runtime library.
    ///
    /// Construction never reads artifacts or starts inference. The paths are
    /// stored without validation; use absolute paths for predictable deployment.
    /// Artifact and runtime errors are reported by the first supported
    /// [`redact`](Self::redact) call. All instances in one process
    /// must use the same compatible ONNX Runtime.
    pub fn new(directory: PathBuf, runtime: PathBuf) -> Self {
        Self {
            model: None,
            directory,
            runtime,
        }
    }
    /// Redact an admitted diagnostic using local rules and model inference.
    ///
    /// A supported request lazily loads the verified model and runtime.
    /// Successful output is truncated to [`MAX_OUTPUT_CHARS`] scalar values.
    ///
    /// # Errors
    ///
    /// Returns [`RedactionError::Input`] when admission fails.
    /// Loading can return [`RedactionError::Artifact`] or
    /// [`RedactionError::Runtime`]; inference can return
    /// [`RedactionError::Inference`] or [`RedactionError::Input`]. Loading and
    /// model-processing failures discard the cached session. Errors do not return
    /// source text: withhold the diagnostic rather than falling back to the original.
    pub fn redact(&mut self, language: &str, text: &str) -> Result<String, RedactionError> {
        if !supported(language, text) {
            return Err(RedactionError::Input);
        }
        let result = (|| {
            if self.model.is_none() {
                self.model = Some(load_model(&self.directory, &self.runtime)?);
            }
            self.model
                .as_mut()
                .ok_or(RedactionError::Runtime)?
                .redact(text)
        })();
        match result {
            Ok(text) => Ok(text.chars().take(MAX_OUTPUT_CHARS).collect()),
            Err(error) => {
                self.model = None;
                Err(error)
            }
        }
    }
}

/// Load the pinned model with the compile-time backend.
///
/// An `ort` build loads the native runtime from `runtime`. A Tract-only build
/// ignores `runtime` and uses packed Q4 weights, or expanded weights with the
/// `tract-expanded` feature.
#[cfg(feature = "ort")]
pub fn load_model(directory: &Path, runtime: &Path) -> Result<Model, RedactionError> {
    Model::load(directory, runtime)
}

#[cfg(all(feature = "tract", not(feature = "ort")))]
/// Load the pinned model with packed or expanded Tract weights.
#[cfg(all(feature = "tract", not(feature = "ort")))]
pub fn load_model(directory: &Path, _runtime: &Path) -> Result<Model, RedactionError> {
    #[cfg(feature = "tract-expanded")]
    {
        Model::load_tract_expanded(directory)
    }
    #[cfg(not(feature = "tract-expanded"))]
    {
        Model::load_tract(directory)
    }
}
/// Return the model-directory and runtime-library paths embedded at compile time.
///
/// An `ort` build needs both `DEFINGERPRINTER_MODEL_DIR` and
/// `DEFINGERPRINTER_RUNTIME` at compile time, otherwise returns `None`. A
/// Tract-only build needs only the model directory and returns an empty runtime
/// path. Nix builds provide immutable paths. These are compile-time settings,
/// not runtime environment-variable overrides. Calling this function does not
/// access files.
///
/// ```
/// if let Some((model, runtime)) = defingerprinter::engine::compiled_paths() {
///     let _redactor = defingerprinter::engine::Redactor::new(model, runtime);
/// }
/// ```
pub fn compiled_paths() -> Option<(PathBuf, PathBuf)> {
    let model = option_env!("DEFINGERPRINTER_MODEL_DIR")?;
    #[cfg(feature = "ort")]
    let runtime = option_env!("DEFINGERPRINTER_RUNTIME")?;
    #[cfg(not(feature = "ort"))]
    let runtime = "";
    Some((model.into(), runtime.into()))
}
/// Check input admission without loading a model or performing redaction.
///
/// Accepts the exact lowercase tags `en`, `es`, `fr`, `de`, `it`, `pt`, `nl`,
/// at most [`MAX_INPUT_BYTES`] bytes and [`MAX_INPUT_CHARS`] scalar values.
/// Characters must belong to Unicode Latin, Common, or Inherited scripts;
/// control characters other than newline, carriage return, and tab are rejected,
/// as are numeric characters other than ASCII digits. Empty text is admitted.
/// This function does not verify that the text is actually written in the
/// declared language.
///
/// ```
/// use defingerprinter::engine::supported;
/// assert!(supported("fr", "Diagnostic synthétique: 123\n"));
/// assert!(!supported("EN", "Synthetic text"));
/// assert!(!supported("en", "text\0"));
/// assert!(!supported("en", "Αlice"));
/// ```
pub fn supported(language: &str, text: &str) -> bool {
    if !["en", "es", "fr", "de", "it", "pt", "nl"].contains(&language)
        || text.len() > MAX_INPUT_BYTES
        || text.chars().count() > MAX_INPUT_CHARS
    {
        return false;
    }
    static SCRIPTS: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let scripts = SCRIPTS.get_or_init(|| {
        regex::Regex::new(r"^[\p{Script=Latin}\p{Script=Common}\p{Script=Inherited}]$")
            .expect("fixed script policy")
    });
    text.chars().all(|c| {
        if c.is_control() && !matches!(c, '\n' | '\r' | '\t') {
            return false;
        }
        if c.is_numeric() && !c.is_ascii_digit() {
            return false;
        }
        scripts.is_match(c.encode_utf8(&mut [0; 4]))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construction_is_lazy_and_direct_failures_discard_session() {
        let mut redactor = Redactor::new("/missing/model".into(), "/missing/runtime".into());
        assert!(redactor.model.is_none());
        for text in ["Αlice", "Alice\0Johnson"] {
            assert!(matches!(
                redactor.redact("en", text),
                Err(RedactionError::Input)
            ));
            assert!(redactor.model.is_none());
        }
        for _ in 0..2 {
            assert!(matches!(
                redactor.redact("en", "Synthetic text"),
                Err(RedactionError::Artifact)
            ));
            assert!(redactor.model.is_none());
        }
    }
}
