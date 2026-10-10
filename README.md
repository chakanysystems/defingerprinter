# Defingerprinter

**Chakany Systems** offline diagnostic redaction library. Rust implementation, versioned C ABI, Common Lisp CFFI bindings, and a self-contained Nix flake. The pinned Rampart q4 model runs locally on a reduced CPU-only ONNX Runtime. No runtime model downloads or network access are required.

## Build with Nix

```sh
nix build .#default
nix flake check
```

Outputs include `bin/defingerprinter`, `lib/libdefingerprinter.dylib` (Darwin) or `.so` (Linux), `include/defingerprinter.h`, and `share/common-lisp/source/defingerprinter/`. Model and runtime are separate store outputs, referenced by immutable compile-time paths. Preserve the complete Nix closure when deploying.

The flake exposes `packages.<system>.{default,defingerprinter,model,runtime}` for `aarch64-darwin`, `aarch64-linux`, and `x86_64-linux`. Darwin arm64 is the locally tested platform; Linux outputs require native build validation. C ABI and CFFI checks perform actual model inference using compiled defaults.

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
  ;; Obtain diagnostic-export consent before enabling or reading diagnostics.
  (defingerprinter:set-consent context t)
  (multiple-value-bind (text status)
      (defingerprinter:protect context "Contact zqx_75@example.com" :language "en")
    (when (eq status :ok)
      (write-line text)))
  (defingerprinter:set-consent context nil))
```

Contexts start disabled and initialize the model lazily. `protect` returns a redacted string and `:ok`, or `nil` and a static status. Consent must be exactly `t` or `nil`. Revocation and closure wait for in-flight protection, including Lisp decoding. Use `with-redactor` or explicitly call `close-redactor`; there is no GC finalizer. Setup failures signal `setup-error` with a static status and a `retry` restart.

For a non-Nix build, provide both absolute `:model-directory` and `:runtime-library` paths to `make-redactor`. Load one shared library and one ONNX Runtime path per process. Restart the process to change the runtime.

## C ABI

See [`include/defingerprinter.h`](include/defingerprinter.h) for the complete version-1 contract. Check `defingerprinter_abi_version`, create a disabled integer handle, enable consent, protect UTF-8 slices into caller-owned storage, then revoke and destroy. Pass `NULL,0` for both artifact paths to use Nix defaults. No allocator ownership crosses the boundary; output is not NUL-terminated. Ignore output on any nonzero status.

Calls serialize per handle. Concurrent calls require distinct output and length storage. Destroyed handles are invalid. Keep the library loaded until all handles and calls finish. Version 1 permits compatible replacements on the same platform and architecture; future incompatible ABI changes require a different version. Model behavior is separately pinned, not an ABI guarantee.

## Policy and privacy

The pipeline combines credential/path/host/opaque-identifier scrubbing, deterministic structured recognizers, mapped sentinel pre-masking, WordPiece NER, overlapping word-aligned windows, span repair, and default-deny placeholders. It redacts every detected label. Placeholder identity is request-local; no rehydration table is exported.

Declare `en`, `es`, `fr`, `de`, `it`, `pt`, or `nl`. Input is bounded to 16,384 UTF-8 bytes and 4,096 Unicode scalars, Latin/Common/Inherited scripts and ASCII digits. Output is capped at 1,024 scalars. Unsupported input, artifact corruption and inference failure return no text, without a heuristic fallback. The caller's language declaration is not language detection.

Redaction is probabilistic, not guaranteed anonymity. Export only short, deliberately constructed diagnostic summaries under explicit consent. Enforce consent before reading diagnostic fields; on failure omit the summary. Coordinate revocation with any caller-side export queue, since a previously returned string is caller-owned. Numeric telemetry is independent of diagnostic text.

Inference uses one CPU thread with spinning, graph optimization and runtime telemetry disabled. No input text is written to disk by the implementation. Revocation drops the model session, but allocator caches and the process-global runtime may retain memory until process exit. Use a supervised subprocess when hard deadlines or process-isolated reclamation are required.

## JSONL CLI

The optional `defingerprinter` executable accepts sequential UTF-8 JSONL requests:

```json
{"op":"consent","enabled":true}
{"op":"protect","language":"en","text":"Contact zqx_75@example.com"}
{"op":"consent","enabled":false}
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

The harness verifies tokenizer fixtures, structured recognizers, multilingual NER, long-window seams, full redaction and consent/protocol boundaries. Node is a reference-test dependency only.

Rampart policy adaptations, model and tokenizer retain National Design Studio attribution under CC BY 4.0. See [`ATTRIBUTION.md`](ATTRIBUTION.md) and [`licenses/CC-BY-4.0.txt`](licenses/CC-BY-4.0.txt). Packages include Rust dependency notices and ONNX Runtime's license and third-party notices. The license for Chakany Systems' original code is pending selection.
