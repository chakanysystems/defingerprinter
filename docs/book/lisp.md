# Common Lisp

```lisp
{{#include ../../examples/lisp.lisp}}
```

Output: `Contact [EMAIL_1]`. `protect` returns the text and a status; `with-redactor` closes the context on exit.

## Run

Install CFFI, Babel, and Bordeaux-Threads. With the [Nix package](quickstart.md):

```sh
PREFIX=$(cd result && pwd -P)
DEFINGERPRINTER_ASD="$PREFIX/share/common-lisp/source/defingerprinter/defingerprinter.asd" \
DEFINGERPRINTER_LIBRARY="$PREFIX/lib/libdefingerprinter.dylib" \
  sbcl --script examples/lisp.lisp
```

Use `.so` instead of `.dylib` on Linux.
