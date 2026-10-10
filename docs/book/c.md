# C

```c
{{#include ../../examples/c.c}}
```

Output: `Contact [EMAIL_1]`. `written` is the output byte count; the output has no NUL terminator.

## Build and run

Use the [Nix package](quickstart.md) and its model/runtime defaults:

```sh
PREFIX=$(cd result && pwd -P)
mkdir -p target
cc examples/c.c -I"$PREFIX/include" -L"$PREFIX/lib" \
  -Wl,-rpath,"$PREFIX/lib" -ldefingerprinter -o target/example-c
./target/example-c
```
