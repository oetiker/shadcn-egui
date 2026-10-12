#!/usr/bin/env bash
# Fail when a workspace crate packages to more than 8 MB. crates.io refuses
# a .crate above 10 MB; the margin leaves room for growth between releases.
#
#   scripts/check-package-size.sh            # limit 8 MB
#   PACKAGE_SIZE_LIMIT=1000 scripts/check-package-size.sh
set -euo pipefail
cd "$(dirname "$0")/.."
limit=${PACKAGE_SIZE_LIMIT:-$((8 * 1024 * 1024))}
meta=$(cargo metadata --format-version 1 --no-deps)
target=$(jq -r .target_directory <<<"$meta")

cargo package --workspace --no-verify --allow-dirty --jobs 4

fail=0
while read -r name version; do
  crate="$target/package/$name-$version.crate"
  size=$(stat -c %s "$crate")
  printf '%10d  %s\n' "$size" "$name"
  if [ "$size" -gt "$limit" ]; then
    echo "too large: $name is $size bytes, the limit is $limit" >&2
    fail=1
  fi
done < <(jq -r '.packages[] | "\(.name) \(.version)"' <<<"$meta")
exit "$fail"
