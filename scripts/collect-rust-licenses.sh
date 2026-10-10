#!/bin/sh
# Run from the Cargo project after dependency vendoring/fetching.
set -eu
DEST=$1
mkdir -p "$DEST/rust"
metadata=$(mktemp)
sources=$(mktemp)
trap 'rm -f "$metadata" "$sources"' EXIT HUP INT TERM
cargo metadata --locked --offline --format-version 1 > "$metadata"
# Keep all interior fields nonempty: shell IFS collapses adjacent tab separators.
jq -r '.packages[] | select(.source != null) | [.name,.version,(if .license == null or .license == "" then "unspecified" else .license end),.manifest_path,(.license_file // "")] | @tsv' "$metadata" > "$sources"
cut -f 1-3 "$sources" > "$DEST/rust-packages.tsv"
while IFS="$(printf '\t')" read -r name version license manifest license_file; do
    dest="$DEST/rust/$name-$version"; mkdir -p "$dest"
    source=$(dirname "$manifest")
    for file in "$source"/LICENSE* "$source"/LICENCE* "$source"/license* "$source"/licence* "$source"/COPYING* "$source"/COPYRIGHT* "$source"/NOTICE* "$source"/notice*; do
        [ ! -f "$file" ] || cp "$file" "$dest/"
    done
    if [ -n "$license_file" ]; then
        case "$license_file" in /*) file="$license_file" ;; *) file="$source/$license_file" ;; esac
        cp "$file" "$dest/"
    fi
done < "$sources"
