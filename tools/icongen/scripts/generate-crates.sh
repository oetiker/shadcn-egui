#!/usr/bin/env bash
# Regenerate the icon crates from the pinned npm packages in tools/icongen/SETS.tsv.
#
#   tools/icongen/scripts/generate-crates.sh <scratch dir> [lib...]
#
# Without libs, every row with ship=yes is generated. Downloads land in
# <scratch dir>/npm/<package>-<version>/ and are read as data; only the
# Hugeicons package is executed (imported by hugeicons-svg.mjs), because its
# icons exist only as JavaScript data.
set -euo pipefail
repo=$(cd "$(dirname "$0")/../../.." && pwd)
scratch=${1:?usage: generate-crates.sh <scratch dir> [lib...]}
shift
want=" ${*:-lucide tabler phosphor remixicon hugeicons} "

slots() {
  case $1 in
    lucide) echo check=check,chevron_down=chevron-down,close=x ;;
    tabler) echo check=check,chevron_down=selector,close=x ;;
    phosphor) echo check=check,chevron_down=caret-down,close=x ;;
    remixicon) echo check=check-line,chevron_down=arrow-down-s-line,close=close-line ;;
    hugeicons) echo check=tick-02,chevron_down=unfold-more,close=cancel-01 ;;
    *) echo "unknown lib $1" >&2; exit 1 ;;
  esac
}

title() {
  case $1 in
    lucide) echo "Lucide" ;;
    tabler) echo "Tabler Icons, outline set" ;;
    phosphor) echo "Phosphor Icons, regular weight" ;;
    remixicon) echo "Remix Icon, pinned to its last Apache-2.0 release and never updated" ;;
    hugeicons) echo "Hugeicons, free stroke-rounded set" ;;
  esac
}

fetch() { # fetch <npm name> <version> -> prints the extraction dir
  local dir="$scratch/npm/$(echo "$1" | tr / _)-$2"
  if [ ! -d "$dir/package" ]; then
    mkdir -p "$dir"
    local tgz
    tgz=$(curl -sf "https://registry.npmjs.org/$1/$2" | jq -r .dist.tarball)
    curl -sfL "$tgz" | tar -xz -C "$dir"
  fi
  echo "$dir"
}

cargo build --release --manifest-path "$repo/tools/icongen/Cargo.toml" --jobs 4
icongen=$(cargo metadata --manifest-path "$repo/tools/icongen/Cargo.toml" --format-version 1 --no-deps | jq -r .target_directory)/release/icongen

tail -n +2 "$repo/tools/icongen/SETS.tsv" | while IFS=$'\t' read -r lib package version glob license ship; do
  case $want in *" $lib "*) ;; *) continue ;; esac
  if [ "$ship" != yes ]; then
    echo "skip $lib (ship=$ship)"
    continue
  fi
  dir=$(fetch "$package" "$version")
  if [ "$lib" = hugeicons ]; then
    svgdir="$scratch/hugeicons-svg-$version"
    node "$repo/tools/icongen/scripts/hugeicons-svg.mjs" "$dir" "$svgdir"
    files=("$svgdir"/*.svg)
  else
    # shellcheck disable=SC2206  # the glob from SETS.tsv is meant to expand
    files=("$dir"/$glob)
  fi
  crate="$repo/crates/shadcn-egui-icons-$lib"
  "$icongen" --crate-dir "$crate" --slots "$(slots "$lib")" \
    --provenance "$(title "$lib") $version ($license), from the npm package $package." "${files[@]}"
  lic=$(find "$dir/package" -maxdepth 1 -type f -iname 'licen[cs]e*' | head -1)
  [ -n "$lic" ] || { echo "$lib: no licence file in $dir/package" >&2; exit 1; }
  cp "$lic" "$crate/LICENSE-ICONS.txt"
done
