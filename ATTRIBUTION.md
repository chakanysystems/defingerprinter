# Attribution

The deterministic recognizers, validators, label sentinels, overlap reconciliation,
NER span repair, and placeholder policy in `src/policy.rs` are adapted from
**Rampart**, by **National Design Studio**, pinned npm package
`@nationaldesignstudio/rampart` **0.1.3**.

- Original project: https://github.com/nationaldesignstudio/rampart
- Original package: https://www.npmjs.com/package/@nationaldesignstudio/rampart/v/0.1.3
- Original source files: `src/heuristics.ts`, `src/validators.ts`, `src/premask.ts`,
  `src/policy.ts`, `src/session.ts`, `src/types.ts`, `src/guard.ts`, and
  `src/ner/classifier.ts`.
- License: **Creative Commons Attribution 4.0 International (CC BY 4.0)**,
  https://creativecommons.org/licenses/by/4.0/ . Preserve the upstream license
  alongside distributed copies.

The model and tokenizer are National Design Studio's `nationaldesignstudio/rampart`,
pinned revision `b1993e4e68b082835b80ffc65acc03325ea2e501`:
https://huggingface.co/nationaldesignstudio/rampart/tree/b1993e4e68b082835b80ffc65acc03325ea2e501 .

## Changes for Seismograph

Seismograph contributors ported the policy to synchronous Rust. Offsets and sentinel
projection use UTF-8 byte ranges instead of JavaScript UTF-16 offsets. Fixed regexes
are cached; invalid UTF-8 ranges and non-finite confidence values are rejected.
IPv6 candidates retain the source grammar and boundaries and receive an additional
native IPv6 parse check. Deterministic-label score-1 spans receive the heuristic
tie-break because the small shared `Span` interface has no source field.

All detected labels are redacted, including CITY, STATE, and ZIP_CODE, using the
empty keep-set already selected by Seismograph's JavaScript client. Placeholder
identity is request-local; NFC normalization additionally unifies canonically
equivalent spellings. No rehydration table is retained or exported. The mapped
pre-mask uses the original trained label sentinels; a separate identity-offset
blank-mask helper is not the inference path.

Additional credential, filesystem path, host, and opaque-identifier scrubbing is
ported from Seismograph's `client/index.js` `preRedact`, in the original replacement
order. These additions are not claims of complete secret or PII detection.

Defingerprinter extracts this native implementation as a Chakany Systems project,
adds a versioned C ABI and Common Lisp bindings, and packages the pinned model
and reduced runtime with Nix. Original Chakany Systems code is licensed under
MIT; see `LICENSE`. Rampart-derived policy material, model and tokenizer are
licensed under CC BY 4.0 with the National Design Studio credit above.
The crate's `MIT AND CC-BY-4.0` metadata records both sets of obligations.

This adaptation does not imply endorsement by National Design Studio.
