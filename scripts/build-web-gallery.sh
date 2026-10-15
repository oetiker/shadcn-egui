#!/usr/bin/env bash
# Builds the browser gallery into <out-dir>: the gallery example for
# wasm32-unknown-unknown, the wasm-bindgen output for it, and
# web/index.html with the wasm size and the version filled in.
set -euo pipefail

out=${1:?usage: scripts/build-web-gallery.sh <out-dir>}
root=$(cd "$(dirname "$0")/.." && pwd)
# Resolve a relative <out-dir> against the caller's directory, before the cd.
mkdir -p "$out"
out=$(cd "$out" && pwd)
cd "$root"

bindgen=${WASM_BINDGEN:-wasm-bindgen}
want=$(awk '/^name = "wasm-bindgen"$/ { getline; gsub(/version = |"/, ""); print; exit }' Cargo.lock)
have=$("$bindgen" --version 2>/dev/null | awk '{ print $2 }' || true)
if [ "$have" != "$want" ]; then
    echo "build-web-gallery: needs wasm-bindgen $want, found ${have:-none}." >&2
    echo "Install it with scripts/install-wasm-bindgen.sh <dir> and set WASM_BINDGEN=<dir>/wasm-bindgen." >&2
    exit 1
fi

version=$(awk -F'"' '/^\[workspace.package\]/ { p = 1 } p && /^version/ { print $2; exit }' Cargo.toml)

cargo build --locked --release --jobs 4 --target wasm32-unknown-unknown \
    --example gallery --features fonts-all,icons-all
target=$(cargo metadata --locked --format-version 1 --no-deps \
    | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')

"$bindgen" --target web --no-typescript --out-dir "$out" \
    "$target/wasm32-unknown-unknown/release/examples/gallery.wasm"
size=$(( $(wc -c < "$out/gallery_bg.wasm") ))
sed -e "s/__WASM_SIZE__/$size/g" -e "s/__VERSION__/$version/g" web/index.html > "$out/index.html"
echo "build-web-gallery: $out, $size bytes of wasm, version $version"
