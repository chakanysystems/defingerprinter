# C

```c
{{#include ../../examples/c.c}}
```

Output: `Contact [EMAIL_1]`. `written` is the output byte count; the output has no NUL terminator.

## Build and run

Use the [Nix package](quickstart.md) and its model/runtime defaults:

```sh
DEV=$(nix build --print-out-paths .#default.dev)
LIB=$(nix build --print-out-paths .#default.lib)
mkdir -p target
cc examples/c.c -I"$DEV/include" -L"$LIB/lib" \
  -Wl,-rpath,"$LIB/lib" -ldefingerprinter -o target/example-c
./target/example-c
```
