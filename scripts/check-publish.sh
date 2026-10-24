#!/usr/bin/env bash
# Verify that the workspace publishes: package every crate and build each
# package against the packages of the crates it depends on.
#
#   scripts/check-publish.sh             # as the CI runs it
#   scripts/check-publish.sh --jobs 4
#
# The dry run serves the workspace crates from a local registry under their
# current version, and cargo treats a registry crate of one version as
# unchanging: it reuses the source an earlier run unpacked and the library
# it built. Between releases the version stays the same while the code
# changes, so the packages would be verified against old code: with a CI
# cache from the 0.6.0 release, the icon crates failed on `IconSlots::all`,
# which core gained after 0.6.0, and a removed API would have passed. Remove
# both before the run.
set -euo pipefail
cd "$(dirname "$0")/.."
meta=$(cargo metadata --format-version 1 --no-deps)
target=$(jq -r .target_directory <<<"$meta")
cargo_home=${CARGO_HOME:-$HOME/.cargo}

while read -r name version; do
  rm -rf "$target/debug/.fingerprint/$name"-*
  # Local registries unpack to registry/src/-<hash>; crates.io's own
  # directory is named after its index and stays untouched.
  rm -rf "$cargo_home/registry/src"/-*/"$name-$version"
done < <(jq -r '.packages[] | "\(.name) \(.version)"' <<<"$meta")

cargo publish --locked --dry-run --workspace "$@"
