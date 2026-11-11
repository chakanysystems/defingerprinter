# C++

Call the C ABI directly:

```cpp
{{#include ../../examples/cpp.cpp}}
```

Output: `Contact [EMAIL_1]`.

## Build and run

Use the [Nix package](quickstart.md) and its model/runtime defaults:

```sh
DEV=$(nix build --print-out-paths .#default.dev)
LIB=$(nix build --print-out-paths .#default.lib)
mkdir -p target
c++ -std=c++17 examples/cpp.cpp -I"$DEV/include" -L"$LIB/lib" \
  -Wl,-rpath,"$LIB/lib" -ldefingerprinter -o target/example-cpp
./target/example-cpp
```
