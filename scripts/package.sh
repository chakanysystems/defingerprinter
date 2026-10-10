#!/bin/sh
# Build machine only. The resulting darwin-arm64 bundle needs no language runtime.
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"
[ "$(uname -s)-$(uname -m)" = Darwin-arm64 ] || { echo 'Verified target: macOS arm64' >&2; exit 1; }
for command in cargo jq shasum tar codesign; do command -v "$command" >/dev/null; done
RUNTIME="$ROOT/target/native-runtime/package"
[ -f "$RUNTIME/libonnxruntime.dylib" ] || { echo 'Build scripts/build-runtime.sh first' >&2; exit 1; }
expected=$(sed -n 's/^runtime_sha256=//p' "$RUNTIME/build-manifest.txt")
[ "$(shasum -a 256 "$RUNTIME/libonnxruntime.dylib" | cut -d ' ' -f 1)" = "$expected" ]
MODEL=${DEFINGERPRINTER_MODEL_DIR:-"$ROOT/models/rampart"}
export CARGO_TARGET_DIR="$ROOT/target/native-rust"
# Portable builds use explicit C ABI paths and executable-relative CLI defaults.
(unset DEFINGERPRINTER_MODEL_DIR DEFINGERPRINTER_RUNTIME; cargo build --locked --release)
BUNDLE="$ROOT/target/defingerprinter-darwin-arm64"
mkdir -p "$BUNDLE/model/onnx" "$BUNDLE/licenses/rust"
cp "$CARGO_TARGET_DIR/release/defingerprinter" "$CARGO_TARGET_DIR/release/libdefingerprinter.dylib" "$BUNDLE/"
codesign --force --sign - "$BUNDLE/defingerprinter"
codesign --force --sign - "$BUNDLE/libdefingerprinter.dylib"
mkdir -p "$BUNDLE/include"
cp "$ROOT/include/defingerprinter.h" "$BUNDLE/include/"
mkdir -p "$BUNDLE/share/common-lisp/source/defingerprinter/tests"
cp defingerprinter.asd "$BUNDLE/share/common-lisp/source/defingerprinter/"
cp "$ROOT/LICENSE" "$BUNDLE/share/common-lisp/source/defingerprinter/"
cp -R lisp "$BUNDLE/share/common-lisp/source/defingerprinter/"
cp tests/cffi.lisp "$BUNDLE/share/common-lisp/source/defingerprinter/tests/"
cp "$RUNTIME/libonnxruntime.dylib" "$BUNDLE/"
cp "$RUNTIME"/licenses/* "$BUNDLE/licenses/"
cp "$RUNTIME/build-manifest.txt" "$BUNDLE/runtime-build-manifest.txt"
cp "$ROOT/ATTRIBUTION.md" "$BUNDLE/"
cp "$ROOT/LICENSE" "$BUNDLE/"
cp "$ROOT/licenses/CC-BY-4.0.txt" "$BUNDLE/licenses/"
cp "$ROOT/README.md" "$BUNDLE/README.md"
for file in config.json tokenizer.json tokenizer_config.json onnx/model_q4.onnx; do cp "$MODEL/$file" "$BUNDLE/model/$file"; done
sh scripts/collect-rust-licenses.sh "$BUNDLE/licenses"
# Validate pinned model bytes and actual inference, using only bundle-relative files.
printf '%s\n' '{"op":"consent","enabled":true}' '{"op":"protect","language":"en","text":"Contact zqx_75@example.com"}' | "$BUNDLE/defingerprinter" > target/native-bundle-smoke.jsonl
jq -e -s 'length == 2 and .[1].ok == true and (.[1].text | contains("zqx_75") | not)' target/native-bundle-smoke.jsonl >/dev/null
(cd "$BUNDLE" && find . -type f ! -name SHA256SUMS -exec shasum -a 256 {} \; | LC_ALL=C sort > SHA256SUMS)
COPYFILE_DISABLE=1 tar -czf "$BUNDLE.tar.gz" -C "$ROOT/target" "$(basename "$BUNDLE")"
echo "Bundle: $BUNDLE"
echo "Archive: $BUNDLE.tar.gz"
wc -c "$BUNDLE/defingerprinter" "$BUNDLE/libdefingerprinter.dylib" "$BUNDLE/libonnxruntime.dylib" "$BUNDLE/model/tokenizer.json" "$BUNDLE/model/onnx/model_q4.onnx" "$BUNDLE.tar.gz"
