#!/usr/bin/env bash
# What each workspace crate would upload to crates.io: a description, the
# SPDX licence its contents need, the licence texts, and no tests or
# snapshots. Run from anywhere inside the repository.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
fail=0
bad() { echo "check-packages: $*" >&2; fail=1; }
data_licence() { awk -F'\t' -v k="$2" -v c="$3" 'NR > 1 && $1 == k { print $c }' "$1"; }
spdx() { if [ "$1" = MIT ]; then echo MIT; else echo "MIT AND $1"; fi; }

meta=$(cargo metadata --no-deps --format-version 1)
while IFS=$'\t' read -r name license has_description; do
  case "$name" in
    shadcn-egui|shadcn-egui-core) want=MIT ;;
    shadcn-egui-font-*) want=$(spdx "$(data_licence tools/fontgen/FONTS.tsv "${name#shadcn-egui-font-}" 4)") ;;
    shadcn-egui-icons-*) want=$(spdx "$(data_licence tools/icongen/SETS.tsv "${name#shadcn-egui-icons-}" 5)") ;;
    *) bad "$name: not a known crate family (core, font, icons, shadcn-egui)"; continue ;;
  esac
  [ "$license" = "$want" ] || bad "$name: license is '$license', expected '$want'"
  [ "$has_description" = yes ] || bad "$name: no description"
  cmp -s LICENSE "crates/$name/LICENSE" || bad "$name: crates/$name/LICENSE is missing or differs from LICENSE"
  if ! files=$(cargo package --list --allow-dirty -p "$name" 2>/dev/null); then
    bad "$name: cargo package --list failed"
    continue
  fi
  for f in Cargo.toml src/lib.rs LICENSE; do
    grep -qx "$f" <<< "$files" || bad "$name: package lacks $f"
  done
  if grep -qE '^(tests|examples|benches)/' <<< "$files"; then
    bad "$name: package contains tests, examples or benches"
  fi
  case "$name" in
    shadcn-egui)
      grep -qx README.md <<< "$files" || bad "$name: package lacks README.md" ;;
    shadcn-egui-font-*)
      grep -qx LICENSE-FONT.txt <<< "$files" || bad "$name: package lacks LICENSE-FONT.txt"
      grep -qE '^fonts/.+\.ttf$' <<< "$files" || bad "$name: package lacks fonts/*.ttf" ;;
    shadcn-egui-icons-*)
      grep -qx LICENSE-ICONS.txt <<< "$files" || bad "$name: package lacks LICENSE-ICONS.txt"
      grep -qx src/icons.bin <<< "$files" || bad "$name: package lacks src/icons.bin" ;;
  esac
done < <(jq -r '.packages[] | [.name, (.license // "-"), (if (.description // "") == "" then "no" else "yes" end)] | @tsv' <<< "$meta")
if [ "$fail" = 0 ]; then echo "check-packages: ok"; fi
exit "$fail"
