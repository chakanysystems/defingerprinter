#!/bin/sh
# Run from the checkout root, or let this script resolve it.
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"

mdbook build
RUSTDOCFLAGS='-D warnings' cargo doc --locked --no-deps
cargo test --locked --doc
cargo check --locked --examples
"${CC:-cc}" -std=c11 -Wall -Wextra -Werror -fsyntax-only -Iinclude examples/c.c
"${CXX:-c++}" -std=c++17 -Wall -Wextra -Werror -fsyntax-only -Iinclude examples/cpp.cpp

if command -v node >/dev/null 2>&1; then
    node --check examples/node.mjs
else
    printf '%s\n' 'SKIP Node.js example: node not installed'
fi
if command -v python3 >/dev/null 2>&1; then
    python3 -m py_compile examples/python.py
else
    printf '%s\n' 'SKIP Python example: python3 not installed'
fi
if command -v go >/dev/null 2>&1; then
    go build -o target/example-go examples/go/main.go
else
    printf '%s\n' 'SKIP Go example: go not installed'
fi
