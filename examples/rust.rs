use defingerprinter::engine::Redactor;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = env::var("DEFINGERPRINTER_MODEL_DIR")?;
    let runtime = env::var("DEFINGERPRINTER_RUNTIME")?;
    let mut redactor = Redactor::new(model.into(), runtime.into());

    let text = redactor.redact("en", "Contact alice@example.com")?;
    println!("{text}");
    Ok(())
}
