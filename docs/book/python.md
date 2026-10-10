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
PREFIX=$(cd result && pwd -P)
python3 examples/python.py "$PREFIX/lib/libdefingerprinter.dylib"
```

Use `.so` instead of `.dylib` on Linux.
