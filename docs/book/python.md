# Python

With the `ctypes` declarations below:

```python
{{#include ../../examples/python.py:28:43}}
```

## Binding setup

```python
{{#include ../../examples/python.py:1:24}}
```

## Run

With the [Nix package](quickstart.md):

```sh
LIB=$(nix build --print-out-paths .#default.lib)
python3 examples/python.py "$LIB/lib/libdefingerprinter.dylib"
```

Use `.so` instead of `.dylib` on Linux.
