# C++

Call the C ABI directly:

```cpp
{{#include ../../examples/cpp.cpp}}
```

Output: `Contact [EMAIL_1]`.

## Build and run

Use the [Nix package](quickstart.md) and its model/runtime defaults:

```sh
PREFIX=$(cd result && pwd -P)
mkdir -p target
c++ -std=c++17 examples/cpp.cpp -I"$PREFIX/include" -L"$PREFIX/lib" \
  -Wl,-rpath,"$PREFIX/lib" -ldefingerprinter -o target/example-cpp
./target/example-cpp
```
