# Go

```go
{{#include ../../examples/go/main.go}}
```

Call the JSONL subprocess interface with Go's standard library; the Nix-built executable uses compiled model and runtime defaults.

```sh
nix build
PREFIX=$(cd result && pwd -P)
go run examples/go/main.go "$PREFIX/bin/defingerprinter"
```
