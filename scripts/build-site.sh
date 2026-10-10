#!/bin/sh
# Build a combined handbook and Rust API site without native inference artifacts.
set -eu
cd "$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"

mkdir -p target
staging=$(mktemp -d "$PWD/target/site.XXXXXXXX")
trap 'rm -rf "$staging"' EXIT HUP INT TERM
export CARGO_TARGET_DIR="$PWD/target/site-cargo"

mdbook build --dest-dir "$staging"
RUSTDOCFLAGS="${RUSTDOCFLAGS:-} -D warnings" cargo doc --locked --no-deps --lib
cargo test --locked --doc
mkdir "$staging/api"
cp -R "$CARGO_TARGET_DIR/doc/." "$staging/api/"
cp docs/api-index.html "$staging/api/index.html"
touch "$staging/.nojekyll"

test -f "$staging/index.html"
test -f "$staging/api/defingerprinter/index.html"
# Replace generated output only after both documentation builds succeed.
rm -rf target/site
mv "$staging" target/site
printf '%s\n' 'Combined documentation: target/site/index.html'
