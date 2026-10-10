//! Bounded JSONL with lazy local inference. No raw diagnostic error output.
use defingerprinter::{
    RedactionError,
    engine::{compiled_paths, supported},
    model::Model,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    io::{self, BufRead, Write},
    path::PathBuf,
    time::Instant,
};

const MAX_LINE: usize = 32768;
#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum Request {
    Protect { language: String, text: String },
    Status,
    Benchmark { iterations: usize },
}
struct State {
    model: Option<Model>,
    directory: PathBuf,
    runtime: PathBuf,
}
impl State {
    fn handle(&mut self, request: Request) -> Value {
        match request {
            Request::Status => {
                json!({"ok":true,"ready":self.model.is_some()})
            }
            Request::Protect { language, text } => {
                if !supported(&language, &text) {
                    return dropped("unsupported_input");
                }
                let result = self.ready().and_then(|model| model.protect(&text));
                match result {
                    Ok(text) => {
                        json!({"ok":true,"text":text.chars().take(1024).collect::<String>()})
                    }
                    Err(_) => {
                        self.model = None;
                        dropped("redaction_failed")
                    }
                }
            }
            Request::Benchmark { iterations } => {
                if !(1..=1000).contains(&iterations) {
                    return dropped("invalid_request");
                }
                self.benchmark(iterations).unwrap_or_else(|_| {
                    self.model = None;
                    dropped("redaction_failed")
                })
            }
        }
    }
    fn ready(&mut self) -> Result<&mut Model, RedactionError> {
        if self.model.is_none() {
            self.model = Some(Model::load(&self.directory, &self.runtime)?);
        }
        self.model.as_mut().ok_or(RedactionError::Runtime)
    }
    fn benchmark(&mut self, iterations: usize) -> Result<Value, RedactionError> {
        let start = Instant::now();
        let model = self.ready()?;
        let load_ms = start.elapsed().as_secs_f64() * 1000.0;
        let text = "The repair failed while processing Alice Johnson's report at 123 Main Street, Boston. Contact alice@example.com or 212-555-0199. The tool used /Users/alice/project and token=sk-synthetic-secret-value.";
        model.protect(text)?;
        let mut durations = Vec::new();
        for _ in 0..iterations {
            let start = Instant::now();
            model.protect(text)?;
            durations.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        durations.sort_by(f64::total_cmp);
        Ok(
            json!({"ok":true,"iterations":iterations,"input_chars":text.chars().count(),"load_ms":load_ms,"median_ms":durations[iterations/2],"p95_ms":durations[(iterations*95/100).min(iterations-1)]}),
        )
    }
}
fn dropped(reason: &str) -> Value {
    json!({"ok":false,"dropped":true,"reason":reason})
}
// Drain oversized records without allocating them. Preserve the next JSONL boundary.
fn read_record(reader: &mut impl BufRead) -> io::Result<Option<Result<Vec<u8>, ()>>> {
    let mut data = Vec::new();
    let mut overflow = false;
    loop {
        let buffer = reader.fill_buf()?;
        if buffer.is_empty() {
            return Ok(if data.is_empty() && !overflow {
                None
            } else {
                Some(if overflow { Err(()) } else { Ok(data) })
            });
        }
        let end = buffer.iter().position(|b| *b == b'\n').map(|i| i + 1);
        let count = end.unwrap_or(buffer.len());
        if data.len() + count > MAX_LINE {
            overflow = true;
            data.clear();
        }
        if !overflow {
            data.extend_from_slice(&buffer[..count]);
        }
        reader.consume(count);
        if end.is_some() {
            return Ok(Some(if overflow { Err(()) } else { Ok(data) }));
        }
    }
}
fn run() -> Result<(), ()> {
    // Runtime and model paths are startup authority, never request fields.
    let executable = std::env::current_exe().map_err(|_| ())?;
    let base = executable.parent().ok_or(())?;
    let mut directory = base.join("model");
    let mut runtime = base.join(if cfg!(target_os = "macos") {
        "libonnxruntime.dylib"
    } else if cfg!(target_os = "windows") {
        "onnxruntime.dll"
    } else {
        "libonnxruntime.so"
    });
    if let Some((model_path, runtime_path)) = compiled_paths() {
        directory = model_path;
        runtime = runtime_path;
    }
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--model-dir") => directory = PathBuf::from(args.next().ok_or(())?),
            Some("--runtime") => runtime = PathBuf::from(args.next().ok_or(())?),
            _ => return Err(()),
        }
    }
    let mut state = State {
        model: None,
        directory,
        runtime,
    };
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    while let Some(record) = read_record(&mut input).map_err(|_| ())? {
        let response = match record {
            Ok(bytes) => match serde_json::from_slice::<Request>(&bytes) {
                Ok(request) => state.handle(request),
                Err(_) => dropped("invalid_request"),
            },
            Err(()) => dropped("invalid_request"),
        };
        serde_json::to_writer(&mut output, &response).map_err(|_| ())?;
        output.write_all(b"\n").map_err(|_| ())?;
        output.flush().map_err(|_| ())?;
    }
    Ok(())
}
fn main() {
    if run().is_err() {
        eprintln!("redactor stopped");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn direct_protection_loads_lazily_and_failure_resets_session() {
        let mut s = State {
            model: None,
            directory: "missing".into(),
            runtime: "missing".into(),
        };
        assert_eq!(s.handle(Request::Status), json!({"ok":true,"ready":false}));
        assert_eq!(
            s.handle(Request::Protect {
                language: "en".into(),
                text: "Alice".into()
            })["reason"],
            "redaction_failed"
        );
        assert!(s.model.is_none());
        assert_eq!(
            s.handle(Request::Benchmark { iterations: 1 })["reason"],
            "redaction_failed"
        );
        assert!(s.model.is_none());
    }
    #[test]
    fn unsupported_scripts_and_bounds() {
        assert!(supported("fr", "José a envoyé une erreur 🙂"));
        assert!(!supported("en", "Αlice"));
        assert!(!supported("ru", "Alice"));
        assert!(!supported("en", &"a".repeat(4097)));
        assert!(!supported("en", "a\0b"));
    }
    #[test]
    fn unsupported_numeric_scripts_drop_before_load() {
        let mut s = State {
            model: None,
            directory: "missing".into(),
            runtime: "missing".into(),
        };
        for text in [
            "١٢٣٤٥٦٧٨٩",
            "१२३४५६७८९",
            "１２３４５６７８９",
            "Alice\u{093e}",
        ] {
            assert_eq!(
                s.handle(Request::Protect {
                    language: "en".into(),
                    text: text.into()
                })["reason"],
                "unsupported_input"
            );
            assert!(s.model.is_none());
        }
    }
    #[test]
    fn bounded_reader_recovers() {
        let mut bytes = vec![b'a'; MAX_LINE + 1];
        bytes.extend_from_slice(b"\n{}\n");
        let mut reader = io::Cursor::new(bytes);
        assert!(read_record(&mut reader).unwrap().unwrap().is_err());
        assert_eq!(read_record(&mut reader).unwrap().unwrap().unwrap(), b"{}\n");
        assert!(read_record(&mut reader).unwrap().is_none());
    }
}
