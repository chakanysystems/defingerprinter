# Statuses and recovery

Every nonzero C status means **omit the diagnostic summary**. The caller owns any output buffer, but must ignore its contents on failure. `written` is reset to zero when valid writable length storage is supplied.

## Version-2 C ABI statuses

| Value | Symbol | Meaning / next action |
| --- | --- | --- |
| 0 | `OK` | Consume exactly `written` UTF-8 bytes |
| 1 | `INVALID_ARGUMENT` | Correct pointer/length or path setup |
| 2 | `INVALID_HANDLE` | Use a current live handle; fix ownership |
| 3 | Reserved | Not returned by ABI 2 |
| 4 | `UNSUPPORTED_INPUT` | Review declared language, encoding, bounds, and script policy |
| 5 | `ARTIFACT_ERROR` | Verify the pinned model files and setup paths |
| 6 | `RUNTIME_ERROR` | Verify native runtime availability and process-wide path selection |
| 7 | `INFERENCE_ERROR` | Omit text and inspect the deployment with synthetic input |
| 8 | `BUFFER_TOO_SMALL` | Allocate at least 4,096 bytes before the next call |
| 9 | `INTERNAL_ERROR` | Omit text; inspect with static diagnostics and a fresh process |

The full C names have the `DEFINGERPRINTER_` prefix. Error categories are intentionally coarse; they are not guaranteed to identify every underlying cause.

The API retains no source text for retry. Allocate the full output capacity up front instead of using an output-sizing probe. Choose retries according to application policy, without logging the original request.

## Rust

`RedactionError` has four variants:

- `Artifact`: invalid pinned artifacts.
- `Runtime`: native runtime unavailable.
- `Inference`: redaction inference failed.
- `Input`: unsupported diagnostic input.

The engine clears its model session after model failures. A later valid call can attempt lazy initialization again. Rust owns the returned `String` on success; on error there is no output string.

## Common Lisp

`redact` returns `nil` and a static status on failure. Use its second value, not truthiness of a guessed result. Setup errors are conditions with a static status and a `retry` restart. Correct setup before invoking the restart; close contexts deterministically.

## JSONL

The executable returns `ok:false`, `dropped:true`, and a static `reason`, not ABI numeric statuses. Native artifact/runtime/inference failures are grouped as `redaction_failed`. Treat transport failures, timeout, and malformed responses as failed redaction too.

## Investigate without real diagnostics

Reproduce with a fixed synthetic summary, verify source/model/runtime revisions, then run the native package and its C ABI/CFFI checks. Report static codes and build provenance.
