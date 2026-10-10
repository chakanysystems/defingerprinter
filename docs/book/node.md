# JavaScript / Node.js

```javascript
{{#include ../../examples/node.mjs}}
```

Call the JSONL subprocess interface with Node's standard library; the Nix-built executable uses compiled model and runtime defaults.

```sh
nix build
PREFIX=$(cd result && pwd -P)
node examples/node.mjs "$PREFIX/bin/defingerprinter"
```
