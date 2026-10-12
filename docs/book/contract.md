# The integration contract

## Input and output

| Property | Contract |
| --- | --- |
| Encoding | UTF-8; ABI lengths count bytes, not characters |
| Language declaration | Exactly `en`, `es`, `fr`, `de`, `it`, `pt`, or `nl` |
| Input byte limit | 16,384 |
| Input Unicode scalar limit | 4,096 |
| Script policy | Latin, Common, and Inherited scripts |
| Numeric characters | ASCII digits only |
| Controls | Only newline, carriage return, and tab are permitted |
| Output | At most 1,024 Unicode scalars |
| C output storage | Allocate at least 4,096 bytes plus separate `size_t written` storage |
| JSONL record limit | 32,768 bytes, including the line terminator |

A language declaration is the caller's claim, not automatic language detection. Validation does not establish the model's accuracy for a particular language. Test representative synthetic diagnostics from your application.

The output limit truncates the redacted result; design summaries short enough to preserve useful context. Unsupported input and artifact/runtime/inference failures return no diagnostic text. Placeholder identity is request-local, and no rehydration map is exported.

## Lifetime

Creation does not load model files. The first valid redaction call loads and verifies artifacts lazily. Close every context deterministically to drop its model session.

The C ABI serializes redaction and destruction per handle. Destruction waits for in-flight work. Each concurrent call needs its own output buffer and length storage. Use separate handles for concurrent inference. Keep the shared library loaded until every live handle and call finishes.

Rust callers own a mutable `Redactor` and arrange their own synchronization. Common Lisp bindings hold their context lock through decoding, so closure waits for that work too.

## Artifact authority

For Nix-built C contexts, pass `NULL,0` for **both** model and runtime paths to use compiled immutable defaults. For a manual build, pass both trusted absolute UTF-8 paths. Paths are setup authority, never fields in an untrusted diagnostic request.

The ONNX Runtime selection is process-global. Load one runtime path per process; restart the process to change it. Keep model files and runtime binaries immutable during execution.

## Export rule

Consume output only after a successful status. On every failure, omit the summary rather than exporting the original. A previously returned string belongs to the caller; apply your application's authorization, export, and retention policy.

See [statuses and recovery](errors.md) for the complete version-2 status table and [deployment](deployment.md) for artifact layout.
