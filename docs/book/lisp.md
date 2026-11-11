# Common Lisp

```lisp
{{#include ../../examples/lisp.lisp}}
```

Output: `Contact [EMAIL_1]`. `redact` returns the text and a status; `with-redactor` closes the context on exit.

## Run

Install CFFI, Babel, and Bordeaux-Threads. With the [Nix package](quickstart.md):

```sh
DEV=$(nix build --print-out-paths .#default.dev)
LIB=$(nix build --print-out-paths .#default.lib)
DEFINGERPRINTER_ASD="$DEV/share/common-lisp/source/defingerprinter/defingerprinter.asd" \
DEFINGERPRINTER_LIBRARY="$LIB/lib/libdefingerprinter.dylib" \
  sbcl --script examples/lisp.lisp
```

Use `.so` instead of `.dylib` on Linux.
