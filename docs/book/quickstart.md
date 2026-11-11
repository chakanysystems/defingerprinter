# Quickstart

From the repository root:

```sh
BIN=$(nix build --print-out-paths .#default.bin)
printf '%s\n' '{"op":"redact","language":"en","text":"Contact alice@example.com"}' \
  | "$BIN/bin/defingerprinter"
```

Response:

```json
{"ok":true,"text":"Contact [EMAIL_1]"}
```

## Use the API

- [Rust](rust.md)
- [C](c.md)
- [C++](cpp.md)
- [Python](python.md)
- [Node.js](node.md)
- [Go](go.md)
- [Common Lisp](lisp.md)

## Local documentation

```sh
nix develop .#docs -c mdbook serve --hostname 127.0.0.1 --port 3000
cargo doc --locked --no-deps
```

Open `http://127.0.0.1:3000` for the handbook or `target/doc/defingerprinter/index.html` for the Rust API.
