# JavaScript / Node.js

```javascript
{{#include ../../examples/node.mjs}}
```

Call the JSONL subprocess interface with Node's standard library; the Nix-built executable uses compiled model and runtime defaults.

```sh
BIN=$(nix build --print-out-paths .#default.bin)
node examples/node.mjs "$BIN/bin/defingerprinter"
```
