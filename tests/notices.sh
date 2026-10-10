#!/bin/sh
# Cargo permits an explicit license_file without an SPDX license expression.
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT HUP INT TERM
mkdir -p "$WORK/bin" "$WORK/relative/legal" "$WORK/absolute/legal"
printf 'relative license fixture\n' > "$WORK/relative/legal/terms.txt"
printf 'absolute license fixture\n' > "$WORK/absolute/legal/terms.txt"
jq -n --arg root "$WORK" '{packages: [
  {name: "relative", version: "1.0", source: "registry+https://example.invalid",
   manifest_path: ($root + "/relative/Cargo.toml"), license: null,
   license_file: "legal/terms.txt"},
  {name: "absolute", version: "1.0", source: "registry+https://example.invalid",
   manifest_path: ($root + "/absolute/Cargo.toml"), license: "",
   license_file: ($root + "/absolute/legal/terms.txt")}
]}' > "$WORK/metadata.json"
cat > "$WORK/bin/cargo" <<'SH'
#!/bin/sh
cat "$NOTICE_TEST_METADATA"
SH
chmod +x "$WORK/bin/cargo"
PATH="$WORK/bin:$PATH" NOTICE_TEST_METADATA="$WORK/metadata.json" \
  sh "$ROOT/scripts/collect-rust-licenses.sh" "$WORK/out"
cmp "$WORK/relative/legal/terms.txt" "$WORK/out/rust/relative-1.0/terms.txt"
cmp "$WORK/absolute/legal/terms.txt" "$WORK/out/rust/absolute-1.0/terms.txt"
echo 'Rust dependency license-file checks passed'
