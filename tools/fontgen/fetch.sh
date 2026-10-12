#!/usr/bin/env bash
# Downloads every font file and licence that FONTS.tsv names for a shipped
# font (OFL-1.1 or Apache-2.0) from google/fonts at the commit in GF_PIN.
# Files land under <downloads-dir> at their google/fonts path. Files already
# there are kept, so a re-run only fetches what is missing. A download goes
# to a temporary file and is moved into place only when curl succeeded, so a
# file exists only after a complete transfer. fontgen checks every font it
# reads (table directory and axis data), which is the guard against a bad file.
#
# Usage: tools/fontgen/fetch.sh <downloads-dir>
set -euo pipefail
out=${1:?usage: tools/fontgen/fetch.sh <downloads-dir>}
here=$(cd "$(dirname "$0")" && pwd)
pin=$(tr -d '[:space:]' < "$here/GF_PIN")
ua='shadcn-egui-fontgen (https://github.com/oetiker/shadcn-egui)'
enc() { printf %s "$1" | sed 's/\[/%5B/g; s/\]/%5D/g'; }
awk -F'\t' 'NR > 1 && ($4 == "OFL-1.1" || $4 == "Apache-2.0") {
    n = split($5, f, ";")
    for (i = 1; i <= n; i++) { sub(/^[0-9]+=/, "", f[i]); print f[i] }
    print $6
}' "$here/FONTS.tsv" | sort -u | while read -r p; do
    dest="$out/$p"
    [ -s "$dest" ] && continue
    mkdir -p "$(dirname "$dest")"
    tmp=$(mktemp "$dest.XXXXXX")
    curl -sfL -A "$ua" -o "$tmp" "https://raw.githubusercontent.com/google/fonts/$pin/$(enc "$p")" || { rm -f "$tmp"; exit 1; }
    chmod 644 "$tmp" && mv "$tmp" "$dest" || { rm -f "$tmp"; exit 1; }
    echo "fetched $p"
done
