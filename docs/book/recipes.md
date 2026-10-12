# Integration recipes

## Export a diagnostic once

Keep collection, redaction, and publication as separate steps. Fail closed at each boundary:

```text
context = create()
summary = construct_short_diagnostic()
result = redact(context, declared_language, summary)
close(context)
if result succeeded:
    export(result.text)
else:
    omit summary
```

Export static error counters separately from text.

## Output queues

Successful output belongs to the caller. Bound queued text and apply your application's export and retention policy at the publication boundary.

## Multiple workers

Use distinct redactors or C handles for concurrent inference. Keep the selected runtime path consistent process-wide. Bound worker count and input queues; the library's one-thread inference setting does not bound the number of sessions your application creates.

For one shared C handle, allocate distinct output and `written` storage for every call. The handle lock serializes native work, but it does not serialize your later reads from shared output memory.

## An unresponsive native call

Use a subprocess when a hard deadline is required. On deadline expiry, omit the summary, terminate the process, and reap it. Start a fresh session for the next request. An in-process thread timeout alone does not stop native inference.

## Repeated diagnostic fragments

Create one short summary per event. Placeholder identity is local to a request, so do not treat equal-looking placeholders across events as stable entity identifiers. Store only the successful redacted text if your retention policy permits it.

## A manual bundle

Resolve model and runtime paths from a trusted installation root before creating contexts. Pass both absolute paths explicitly to C or Lisp. With Rust, construct `Redactor` with those paths. With the CLI, supply setup-time flags or use its bundle-relative layout. Preserve pinned artifacts and notices as a unit.
