# The Defingerprinter Handbook

Defingerprinter is an offline diagnostic-redaction library. Use it to redact a short, deliberately constructed diagnostic summary before export. The Rust implementation combines deterministic scrubbing with the pinned Rampart q4 named-entity model, running on a reduced CPU-only ONNX Runtime.

**Start with [the quickstart](quickstart.md)**, then choose your interface:

| Your application | Integration | Dependencies at runtime |
| --- | --- | --- |
| [Rust](rust.md) | `engine::Redactor` | Model files and ONNX Runtime |
| [C](c.md), [C++](cpp.md) | Version-2 C ABI | Shared library, model, runtime |
| [Python](python.md) | Standard-library `ctypes` example | Python plus native artifacts |
| [Common Lisp](lisp.md) | Included ASDF / CFFI bindings | CFFI, Babel, Bordeaux-Threads plus native artifacts |
| [Node.js](node.md), [Go](go.md) | Supervised JSONL subprocess examples | Your language runtime plus the executable closure |
| [Something else](jsonl.md) | UTF-8 JSONL over stdin/stdout | The executable closure |

The Python, C++, Node.js, and Go examples are integration recipes, not separately published SDKs. Rust, the C ABI, the JSONL executable, and Common Lisp bindings are included in this repository. Examples throughout this book use synthetic diagnostics.

The native examples close their sessions before printing successful output. The subprocess examples validate responses and wait for clean process exit before printing.

## The lifecycle, in one line

**Create → protect → close → export only success.**

On a redaction failure, omit the diagnostic summary. Redaction is probabilistic and does not guarantee anonymity. See [the integration contract](contract.md) and [privacy and integration responsibilities](privacy.md).

## What is pinned?

The model artifacts and runtime sources are pinned by the Nix build. The shared-library ABI is independently versioned. ABI compatibility does not promise identical redaction output across model or policy revisions. Preserve the complete Nix closure when deploying.

The handbook is built with mdBook; Rust API documentation is generated separately with rustdoc. Both are built from this checkout, without publishing anything to a hosting service.
