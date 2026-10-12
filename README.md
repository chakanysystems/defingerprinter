# Defingerprinter

Defingerprinter is an offline diagnostic redaction library. It's implemented in Rust, with a versioned C ABI, Common Lisp CFFI bindings, and a self-contained Nix flake. The pinned Rampart q4 model runs locally on a reduced CPU-only ONNX Runtime. No runtime model downloads or network access are required.

## Policy and privacy

The pipeline combines credential/path/host/opaque-identifier scrubbing, deterministic structured recognizers, mapped sentinel pre-masking, WordPiece NER, overlapping word-aligned windows, span repair, and default-deny placeholders. It redacts every detected label. Placeholder identity is request-local; no rehydration table is exported.

Declare `en`, `es`, `fr`, `de`, `it`, `pt`, or `nl`. Input is bounded to 16,384 UTF-8 bytes and 4,096 Unicode scalars, Latin/Common/Inherited scripts and ASCII digits. Output is capped at 1,024 scalars. Unsupported input, artifact corruption and inference failure return no text, without a heuristic fallback. The caller's language declaration is not language detection.

Redaction is probabilistic, not guaranteed anonymity. Prefer short, deliberately constructed diagnostic summaries; on failure omit the summary. Callers choose appropriate authorization, export, and retention policies. Previously returned strings are caller-owned. Numeric telemetry is independent of diagnostic text.

Inference uses one CPU thread with spinning, graph optimization and runtime telemetry disabled. No input text is written to disk by the implementation. Closure drops the model session, but allocator caches and the process-global runtime may retain memory until process exit. Use a supervised subprocess when hard deadlines or process-isolated reclamation are required.

## Build with Nix

```sh
nix build .#default
nix flake check
```

Outputs include `bin/defingerprinter`, `lib/libdefingerprinter.dylib` (Darwin) or `.so` (Linux), `include/defingerprinter.h`, and `share/common-lisp/source/defingerprinter/`. Model and runtime are separate store outputs, referenced by immutable compile-time paths. Preserve the complete Nix closure when deploying.

The flake exposes `packages.<system>.{default,defingerprinter,model,runtime,book,rustdoc,site}` for `aarch64-darwin`, `aarch64-linux`, and `x86_64-linux`. Darwin arm64 is the locally tested platform; Linux outputs require native build validation. C ABI and CFFI checks perform actual model inference using compiled defaults.

Import the local repository from another flake:

```nix
{
  inputs.defingerprinter.url = "git+file:///absolute/path/to/defingerprinter?ref=trunk";
  outputs = { self, defingerprinter }: {
    packages.aarch64-darwin.default =
      defingerprinter.packages.aarch64-darwin.default;
  };
}
```

Use the consumer's lockfile to pin a revision. `trunk` is this project's main bookmark. No published remote is assumed.

## Common Lisp

Install the ASDF sources with CFFI, Babel and Bordeaux-Threads. Load the shared library by its trusted absolute path:

```lisp
(require :asdf)
(asdf:load-asd #P"/absolute/store/output/share/common-lisp/source/defingerprinter/defingerprinter.asd")
(asdf:load-system "defingerprinter")
(defingerprinter:load-library "/absolute/store/output/lib/libdefingerprinter.dylib")
(defingerprinter:with-redactor (context)
  (multiple-value-bind (text status)
      (defingerprinter:redact context "Contact zqx_75@example.com" :language "en")
    (when (eq status :ok)
      (write-line text))))
```

Contexts initialize the model lazily on the first valid redaction call. `redact` returns a redacted string and `:ok`, or `nil` and a static status. Closure drops the session and waits for in-flight redaction, including Lisp decoding. Use `with-redactor` or explicitly call `close-redactor`; there is no GC finalizer. Setup failures signal `setup-error` with a static status and a `retry` restart.

For a non-Nix build, provide both absolute `:model-directory` and `:runtime-library` paths to `make-redactor`. Load one shared library and one ONNX Runtime path per process. Restart the process to change the runtime.

## C ABI

Release 0.1.0 uses the version-2 contract in [`include/defingerprinter.h`](include/defingerprinter.h). Check `defingerprinter_abi_version`, create an integer handle, redact UTF-8 slices into caller-owned storage, then destroy it. Pass `NULL,0` for both artifact paths to use Nix defaults. No allocator ownership crosses the boundary; output is not NUL-terminated. Ignore output on any nonzero status.

Calls serialize per handle. Destruction drops the session and waits for in-flight work. Concurrent calls require distinct output and length storage. Destroyed handles are invalid. Keep the library loaded until all handles and calls finish. Version 2 permits compatible replacements on the same platform and architecture; future incompatible ABI changes require a different version. Status value 3 is reserved; all other status values retain their numbering. Model behavior is separately pinned, not an ABI guarantee.

## JSONL CLI

The optional `defingerprinter` executable accepts sequential UTF-8 JSONL requests:

```json
{"op":"redact","language":"en","text":"Contact zqx_75@example.com"}
{"op":"status"}
```

Use only `text` from an `ok:true` response. On failure, malformed response, EOF or timeout, omit the summary. Records are bounded to 32,768 bytes. Set explicit artifact paths with `--model-dir PATH --runtime PATH` for manual builds; portable bundle defaults resolve relative to the executable. `{"op":"benchmark","iterations":100}` measures a fixed synthetic diagnostic locally.

## Development and notices

```sh
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

Actual-model Cargo tests require `RAMPART_MODEL_DIR` and `RAMPART_RUNTIME` and are explicitly ignored in the ordinary run. See [`docs/runtime-build.md`](docs/runtime-build.md) for the reduced runtime and optional macOS bundle scripts.

For build-time parity against the pinned JavaScript reference, set `RAMPART_REFERENCE_DIR` to a directory containing `node_modules` with `@nationaldesignstudio/rampart` 0.1.3 and `@huggingface/transformers` 4.3.1. Run:

```sh
RAMPART_REFERENCE_DIR=/absolute/reference-install \
RAMPART_MODEL_DIR=/absolute/model \
RAMPART_RUNTIME=/absolute/libonnxruntime.dylib \
node scripts/test-reference-parity.mjs
```

## Handbook and API documentation

The [Defingerprinter Handbook](docs/book/index.md) includes runnable Rust, C, C++, Python, Node.js, Go, and Common Lisp guides, plus privacy, error handling, deployment, and JSONL integration recipes. The language chapters include the checked-in sources from `examples/`.

```sh
nix build .#site --out-link result-site
```

Open `result-site/index.html` for the handbook or `result-site/api/defingerprinter/index.html` for Rust API documentation. The sandboxed build uses pinned tools and vendored Cargo dependencies, checks doctests, and needs no model or runtime artifacts. Separate outputs are available as `.#book` and `.#rustdoc`.

The flake CI workflow discovers and builds packages, checks, development shells, and formatters, then publishes the built site through GitHub Pages. See [publishing instructions](docs/book/development.md#combined-site-and-github-pages).

### License

Original Chakany Systems code is licensed under [MIT](LICENSE). Rampart-derived policy material, model and tokenizer are licensed under CC BY 4.0, credited to National Design Studio. See [`ATTRIBUTION.md`](ATTRIBUTION.md) and [`licenses/CC-BY-4.0.txt`](licenses/CC-BY-4.0.txt). Cargo metadata records `MIT AND CC-BY-4.0`; packages include both notices, Rust dependency notices and ONNX Runtime's license and third-party notices.
