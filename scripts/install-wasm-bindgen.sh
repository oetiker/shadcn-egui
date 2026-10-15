#!/usr/bin/env bash
# Downloads the prebuilt wasm-bindgen CLI whose version matches the
# wasm-bindgen crate in Cargo.lock and puts it at <dir>/wasm-bindgen.
# A CLI of another version fails on the generated module with an error
# that does not name the version, so the version comes from the lock file
# that Dependabot updates.
set -euo pipefail

dir=${1:?usage: scripts/install-wasm-bindgen.sh <dir>}
root=$(cd "$(dirname "$0")/.." && pwd)
version=$(awk '/^name = "wasm-bindgen"$/ { getline; gsub(/version = |"/, ""); print; exit }' "$root/Cargo.lock")
[ -n "$version" ] || { echo "install-wasm-bindgen: no wasm-bindgen in Cargo.lock" >&2; exit 1; }

name=wasm-bindgen-$version-x86_64-unknown-linux-musl
mkdir -p "$dir"
curl -fsSL "https://github.com/wasm-bindgen/wasm-bindgen/releases/download/$version/$name.tar.gz" \
    | tar -xz -C "$dir" --strip-components 1 "$name/wasm-bindgen"
"$dir/wasm-bindgen" --version
