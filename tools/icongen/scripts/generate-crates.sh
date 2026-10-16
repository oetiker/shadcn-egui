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
    *) echo "generate-crates: unknown lib $1" >&2; return 1 ;;
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

die() { echo "generate-crates: $*" >&2; exit 1; }

# fetch <npm name> <version>: sets $pkgdir to the extraction dir. It is called
# directly, never through $(...), so that die ends the whole script. The
# tarball is checked against the registry's dist.integrity (sha512) before it
# is extracted, and extracted into a temporary dir that is moved into place,
# so <dir>/package exists only after a complete extraction.
fetch() {
  local name=$1 version=$2 meta tgz integrity actual tmp
  pkgdir="$scratch/npm/$(echo "$name" | tr / _)-$version"
  [ -d "$pkgdir/package" ] && return 0
  meta=$(curl -sf "https://registry.npmjs.org/$name/$version") || die "$name $version: registry lookup failed"
  tgz=$(jq -er .dist.tarball <<<"$meta") || die "$name $version: no tarball URL in the registry reply"
  integrity=$(jq -er .dist.integrity <<<"$meta") || die "$name $version: no dist.integrity in the registry reply"
  case $integrity in sha512-*) ;; *) die "$name $version: unsupported integrity '$integrity'" ;; esac
  mkdir -p "$scratch/npm"
  tmp=$(mktemp -d "$scratch/npm/.fetch.XXXXXX") || die "$name $version: cannot create a temporary dir"
  curl -sfL -o "$tmp/pkg.tgz" "$tgz" || { rm -rf "$tmp"; die "$name $version: download failed"; }
  actual=sha512-$(openssl dgst -sha512 -binary "$tmp/pkg.tgz" | base64 -w0)
  [ "$actual" = "$integrity" ] || { rm -rf "$tmp"; die "$name $version: integrity mismatch (registry $integrity, got $actual)"; }
  mkdir "$tmp/x"
  tar -xzf "$tmp/pkg.tgz" -C "$tmp/x" || { rm -rf "$tmp"; die "$name $version: extraction failed"; }
  [ -d "$tmp/x/package" ] || { rm -rf "$tmp"; die "$name $version: the tarball has no package/ dir"; }
  rm -rf "$pkgdir"
  mv "$tmp/x" "$pkgdir"
  rm -rf "$tmp"
}

# Every lib asked for must be known to slots() and have a row in SETS.tsv.
for lib in $want; do
  slots "$lib" >/dev/null || exit 1
  grep -q "^$lib	" "$repo/tools/icongen/SETS.tsv" || die "$lib has no row in SETS.tsv"
done

cargo build --release --manifest-path "$repo/tools/icongen/Cargo.toml" --jobs 4
icongen=$(cargo metadata --manifest-path "$repo/tools/icongen/Cargo.toml" --format-version 1 --no-deps | jq -r .target_directory)/release/icongen

tail -n +2 "$repo/tools/icongen/SETS.tsv" | while IFS=$'\t' read -r lib package version glob license ship; do
  case $want in *" $lib "*) ;; *) continue ;; esac
  if [ "$ship" != yes ]; then
    echo "skip $lib (ship=$ship)"
    continue
  fi
  fetch "$package" "$version"
  dir=$pkgdir
  if [ "$lib" = hugeicons ]; then
    svgdir="$scratch/hugeicons-svg-$version"
    # Render into a fresh directory and move it into place only on success,
    # so files of an earlier run are never read as icons.
    mkdir -p "$scratch"
    svgtmp=$(mktemp -d "$scratch/.hugeicons-svg.XXXXXX") || die "hugeicons: cannot create a temporary dir"
    node "$repo/tools/icongen/scripts/hugeicons-svg.mjs" "$dir" "$svgtmp" || { rm -rf "$svgtmp"; die "hugeicons: SVG rendering failed"; }
    rm -rf "$svgdir"
    mv "$svgtmp" "$svgdir"
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
