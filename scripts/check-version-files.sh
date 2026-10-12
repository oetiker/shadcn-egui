#!/usr/bin/env bash
# Every workspace crate has its version_files entries in
# .github/repo-infra.json: one for its Cargo.lock entry, and one for its
# line in [workspace.dependencies] when Cargo.toml has one. Create release
# PR rewrites only what is listed there, so a crate left out (a new font or
# icon set) keeps the old version and the publish fails under --locked.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
config=.github/repo-infra.json
fail=0
while read -r name; do
  jq -e --arg n "$name" '.version_files[]
      | select(.path == "Cargo.lock" and (.pattern | startswith("^name = \"" + $n + "\"\n")))' \
    "$config" > /dev/null \
    || { echo "check-version-files: $config has no Cargo.lock entry for $name" >&2; fail=1; }
  if grep -q "^$name = { path = " Cargo.toml; then
    jq -e --arg n "$name" '.version_files[]
        | select(.path == "Cargo.toml" and (.pattern | startswith("^" + $n + " = ")))' \
      "$config" > /dev/null \
      || { echo "check-version-files: $config has no Cargo.toml entry for $name" >&2; fail=1; }
  fi
done < <(cargo metadata --no-deps --format-version 1 | jq -r '.packages[].name')
if [ "$fail" = 0 ]; then echo "check-version-files: ok"; fi
exit "$fail"
