# Rust

```rust,ignore
{{#include ../../examples/rust.rs}}
```

Output: `Contact [EMAIL_1]`. Reuse the redactor for further calls; its session is released when dropped.

## Setup

Add the checkout to your application's `Cargo.toml`:

```toml
[dependencies]
defingerprinter = { path = "/path/to/defingerprinter" }
```

The example reads the model directory and ONNX Runtime library path from the environment. From this checkout, Nix supplies both:

```sh
nix develop -c cargo run --locked --example rust
```
