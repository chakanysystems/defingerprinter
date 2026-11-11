# Go

```go
{{#include ../../examples/go/main.go}}
```

Call the JSONL subprocess interface with Go's standard library; the Nix-built executable uses compiled model and runtime defaults.

```sh
BIN=$(nix build --print-out-paths .#default.bin)
go run examples/go/main.go "$BIN/bin/defingerprinter"
```
