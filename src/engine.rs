//! Shared input policy and consent-controlled local model session.
use crate::{RedactionError, model::Model};
use std::path::PathBuf;

pub const MAX_INPUT_BYTES: usize = 16_384;
pub const MAX_INPUT_CHARS: usize = 4096;
pub const MAX_OUTPUT_CHARS: usize = 1024;
pub const OUTPUT_CAPACITY: usize = MAX_OUTPUT_CHARS * 4;

pub struct Redactor {
    enabled: bool,
    model: Option<Model>,
    directory: PathBuf,
    runtime: PathBuf,
}
impl Redactor {
    /// Construction never reads artifacts or starts inference.
    pub fn new(directory: PathBuf, runtime: PathBuf) -> Self {
        Self {
            enabled: false,
            model: None,
            directory,
            runtime,
        }
    }
    pub fn enabled(&self) -> bool {
        self.enabled
    }
    pub fn set_consent(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.model = None;
        }
    }
    /// Call only after checking consent, before examining the caller's input.
    pub fn protect(&mut self, language: &str, text: &str) -> Result<String, RedactionError> {
        if !self.enabled || !supported(language, text) {
            return Err(RedactionError::Input);
        }
        let result = (|| {
            if self.model.is_none() {
                self.model = Some(Model::load(&self.directory, &self.runtime)?);
            }
            self.model
                .as_mut()
                .ok_or(RedactionError::Runtime)?
                .protect(text)
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
/// Nix builds embed immutable model/runtime paths for the host's CFFI loader.
pub fn compiled_paths() -> Option<(PathBuf, PathBuf)> {
    Some((
        option_env!("DEFINGERPRINTER_MODEL_DIR")?.into(),
        option_env!("DEFINGERPRINTER_RUNTIME")?.into(),
    ))
}
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
