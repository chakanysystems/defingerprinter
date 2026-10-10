# Development and documentation

## Combined site and GitHub Pages

```sh
nix build .#site --out-link result-site
```

Open `result-site/index.html` for the handbook and `result-site/api/defingerprinter/index.html` for Rust API documentation. The flake builds both in the sandbox with pinned tools and vendored Cargo dependencies, treats rustdoc warnings as errors, and checks doctests. Documentation builds need no model or ONNX Runtime artifacts.

The site is configured for `https://chakanysystems.github.io/defingerprinter/`, including 404 assets under the project subdirectory. The flake CI workflow discovers packages, checks, and development shells from the flake outputs and builds them on native runners. After all build jobs pass, it publishes the built `site` output through GitHub Pages on pushes to `trunk` or a manual run.

In the repository's **Settings → Pages**, select **GitHub Actions** as the publishing source. The `github-pages` environment rules must permit `trunk`.

## Separate documentation outputs

```sh
nix build .#book --out-link result-book
nix build .#rustdoc --out-link result-rustdoc
```

Open `result-book/index.html` or `result-rustdoc/defingerprinter/index.html`. The standalone handbook uses relative asset paths.

Source lives in `docs/book/` and `book.toml`. Language chapters include checked-in files from `examples/`; edit the runnable source rather than duplicating it in Markdown. Missing chapters fail the build.

For a local editing server:

```sh
nix develop .#docs -c mdbook serve --hostname 127.0.0.1 --port 3000
```

Rustdoc covers public APIs and compiles its examples. Inference examples use `no_run` because execution requires model artifacts and runtime paths.

## Documentation checks

```sh
nix develop -c sh scripts/check-docs.sh
```

The script builds mdBook and strict rustdoc, compiles rustdoc examples, checks Cargo examples, and syntax-checks C/C++ sources. It checks Node.js, Python, and Go examples when those toolchains are installed. These are build/syntax checks, not a substitute for actual-model integration runs.

## Library checks

```sh
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
nix flake check
```

Actual-model Cargo tests are explicitly ignored in ordinary runs and require `RAMPART_MODEL_DIR` and `RAMPART_RUNTIME`. C ABI and Common Lisp Nix checks use the compiled artifacts for real inference. See the repository README for reference-parity testing.

Documentation configuration follows the [official mdBook configuration reference](https://rust-lang.github.io/mdBook/format/configuration/index.html).
