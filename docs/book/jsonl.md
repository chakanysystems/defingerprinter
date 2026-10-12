# Any language: JSONL

Run the `defingerprinter` executable and exchange one UTF-8 JSON object per line. It reads sequentially from stdin and flushes one response per request to stdout. The model loads lazily on the first valid redaction call.

## Requests

| Operation | Request | Successful response |
| --- | --- | --- |
| Redact | `{"op":"redact","language":"en","text":"synthetic diagnostic"}` | `{"ok":true,"text":"redacted diagnostic"}` |
| Inspect | `{"op":"status"}` | `{"ok":true,"ready":false}` before loading; `{"ok":true,"ready":true}` after successful redaction |
| Benchmark | `{"op":"benchmark","iterations":100}` | Timing fields with `ok:true` |

`status` reports whether a model session is loaded. It does not initialize the model. Benchmarking accepts 1 through 1,000 iterations and measures a fixed synthetic diagnostic. Its fields include `iterations`, `input_chars`, `load_ms`, `median_ms`, and `p95_ms`; it is a local development operation rather than ordinary diagnostic export.

Request objects reject unknown fields and invalid types. Use JSON serialization, not string interpolation. Embedded newlines in `text` must be JSON-escaped, so every record occupies one physical line.

## Failures

```json
{"ok":false,"dropped":true,"reason":"unsupported_input"}
```

Reasons include `unsupported_input`, `redaction_failed`, and `invalid_request`. They are static categories, not the original input. Accept diagnostic output only from a valid redaction response with `ok:true` and a string `text`.

## Supervision checklist

1. Start a trusted absolute executable path without a shell.
2. Apply a finite deadline to the process and round trips.
3. Serialize transactions; there are no request IDs.
4. Bound UTF-8 request and response records to 32,768 bytes, including the newline.
5. Validate the exact response keys and types expected for each operation.
6. On failure, timeout, malformed JSON, oversized output, or EOF, omit the summary, terminate if needed, and reap the child.
7. Close stdin after the final response and require clean EOF and successful process exit before publishing.

See [Node.js](node.md) and [Go](go.md) for complete standard-library examples.

## Manual artifacts

The Nix executable has compiled defaults. A manual bundle resolves default artifacts relative to the executable, or accepts setup-time flags:

```sh
/absolute/path/to/defingerprinter \
  --model-dir /absolute/path/to/model \
  --runtime /absolute/path/to/libonnxruntime.dylib
```

Pass artifact paths at startup, never in per-diagnostic requests. Use `.so` on Linux.
