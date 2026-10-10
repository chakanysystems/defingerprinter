# Privacy and integration responsibilities

## Application policy

Callers choose the authorization, export, and retention policy appropriate to their application. Create a redactor, protect the diagnostic, and consume only successful output.

## Construct a small summary

Prefer a short description of the failing operation and its coarse outcome. Evaluate redaction coverage with representative synthetic fixtures.

The pipeline scrubs credentials, paths, hosts, and opaque identifiers; applies structured recognizers and sentinel pre-masking; runs WordPiece NER in overlapping word-aligned windows; repairs spans; and emits default-deny placeholders for all detected labels. Detection is probabilistic and does not guarantee anonymity. Evaluate omissions and false positives before export.

## Lifetime and memory

Destroying a C handle waits for in-flight model work and drops the session. Common Lisp closure also waits for Lisp decoding. Successful strings belong to the caller; manage queued and retained output according to your application policy.

The implementation does not write input text to disk. It uses one CPU thread, with spinning, graph optimization, and runtime telemetry disabled. Review application logging, crash reports, and tracing separately.

Allocator caches and the process-global runtime may retain memory after session closure. Use a supervised subprocess when you require a hard deadline or process-isolated reclamation. Termination does not guarantee secure memory wiping.

## Failure reporting

Report static status codes and coarse counters. Omit the summary on failure. The examples print static failures and export only validated successful output.
