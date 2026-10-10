//! Verify every pinned artifact before any runtime loading or inference.
use defingerprinter::{
    RedactionError,
    model::{ARTIFACTS, Model},
};
use std::{
    env, fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = env::temp_dir().join(format!(
            "seismograph-model-verification-{}-{suffix}",
            std::process::id()
        ));
        fs::create_dir_all(path.join("onnx")).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn assert_artifact_failure(path: &Path) {
    let error = match Model::load(path, Path::new("/missing/private-runtime-fixture")) {
        Ok(_) => panic!("unverified artifacts were accepted"),
        Err(error) => error,
    };
    assert!(
        matches!(error, RedactionError::Artifact),
        "artifact failure must precede runtime loading"
    );
    assert!(!error.to_string().contains("private-runtime-fixture"));
    assert!(!error.to_string().contains(&path.display().to_string()));
}

#[test]
fn missing_empty_and_oversized_artifacts_are_rejected_before_runtime() {
    let directory = Directory::new();
    assert_artifact_failure(&directory.0);
    fs::write(directory.0.join("config.json"), b"").unwrap();
    assert_artifact_failure(&directory.0);
    let file = fs::File::create(directory.0.join("config.json")).unwrap();
    file.set_len(20_000_001).unwrap();
    assert_artifact_failure(&directory.0);
}

#[test]
#[ignore = "requires pinned model; set RAMPART_MODEL_DIR"]
fn single_byte_corruption_of_every_artifact_is_rejected() {
    let source = PathBuf::from(env::var("RAMPART_MODEL_DIR").expect("RAMPART_MODEL_DIR required"));
    let directory = Directory::new();
    for (name, _) in ARTIFACTS {
        fs::copy(source.join(name), directory.0.join(name)).unwrap();
    }
    for (name, _) in ARTIFACTS {
        let path = directory.0.join(name);
        let mut bytes = fs::read(&path).unwrap();
        assert!(!bytes.is_empty());
        let index = bytes.len() / 2;
        bytes[index] ^= 1;
        fs::write(&path, &bytes).unwrap();
        assert_artifact_failure(&directory.0);
        fs::copy(source.join(name), path).unwrap();
    }
}
