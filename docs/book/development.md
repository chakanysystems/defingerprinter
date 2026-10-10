# Development and documentation

## Generate the handbook

```sh
nix develop -c mdbook build
nix develop -c mdbook serve --hostname 127.0.0.1 --port 3000
```

Static HTML is written to `target/book/`. The book has local search, light/dark themes, code-copy controls, and printable HTML from mdBook, with a small custom stylesheet. Source lives in `docs/book/` and `book.toml`.

For a reproducible static package:

```sh
nix build .#book --out-link result-book
```

Open `result-book/index.html` or deploy that directory to your static host. Choose your own host and publishing policy; these commands do not publish the documentation.

The language chapters include files from `examples/` directly. Edit the runnable source once rather than duplicating examples into Markdown. Missing chapters fail the build because `create-missing` is disabled.

## Generate Rust API documentation

```sh
RUSTDOCFLAGS='-D warnings' cargo doc --locked --no-deps
cargo test --locked --doc
```

Open `target/doc/defingerprinter/index.html`. Rustdoc covers the public integration APIs and compiles its examples. Inference examples are `no_run` because real model artifacts and runtime paths are setup requirements.

## Combined site and GitHub Pages

```sh
nix develop .#docs -c sh scripts/build-site.sh
```

Open `target/site/index.html` for the handbook and `target/site/api/defingerprinter/index.html` for the API. The documentation shell contains the pinned build tools without initializing or downloading model/runtime artifacts.

The `Documentation` workflow in `.github/workflows/pages.yml` builds this combined site on pushes to `trunk`, then deploys it through GitHub Pages. You can also run it manually from the Actions tab on `trunk`.

In the repository's **Settings → Pages**, select **GitHub Actions** as the publishing source. After enabling Pages and pushing the workflow and documentation commits to `trunk`, the expected site is `https://chakanysystems.github.io/defingerprinter/`, with API docs under `api/defingerprinter/`.

The workflow derives the site's base path from GitHub Pages metadata so the project subdirectory and 404 assets resolve correctly. Deployments use the `github-pages` environment and pinned action revisions. Publishing requires the repository's Pages settings and environment rules to permit that branch.

For a local build with the same project subdirectory:

```sh
MDBOOK_OUTPUT__HTML__SITE_URL=/defingerprinter/ \
  nix develop .#docs -c sh scripts/build-site.sh
```

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
