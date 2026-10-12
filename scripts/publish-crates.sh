#!/usr/bin/env bash
# Publishes every crate of the workspace to crates.io in dependency order:
# shadcn-egui-core, then the font and icon crates, then shadcn-egui. Each
# version crates.io already has is skipped, so a run that stopped part-way
# is finished by running it again until it prints "all published".
#
# crates.io rate limits (read 2026-10-11, see
# docs/superpowers/notes/0.4/p0d-crates-io.md):
#   new crates:          burst 5, then one per 10 min
#   new versions:        burst 30, then one per 1 min
# The first release creates 33 new crates, so it hits HTTP 429 after the
# burst. The script then waits until the time crates.io names ("try again
# after …", else 10 min) and retries the same crate, so one run publishes
# all crates (about 4 h 40 min for the first release).
#
# Usage, from anywhere in the repository:
#   CARGO_REGISTRY_TOKEN=… scripts/publish-crates.sh   publish
#   DRY_RUN=1 scripts/publish-crates.sh                ask crates.io, publish nothing
#   PLAN_ONLY=1 scripts/publish-crates.sh              print the order, no network
#   NO_WAIT=1 scripts/publish-crates.sh                on HTTP 429 exit 2 instead of waiting
# Exit status: 0 done, 2 rate limit with NO_WAIT=1, 1 any other error.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
ua='shadcn-egui-release (https://github.com/oetiker/shadcn-egui)'

meta=$(cargo metadata --no-deps --format-version 1)
mapfile -t members < <(jq -r '.packages[] | select(.publish != []) | .name' <<< "$meta" | sort)
mapfile -t versions < <(jq -r '.packages[] | select(.publish != []) | .version' <<< "$meta" | sort -u)
if [ "${#versions[@]}" != 1 ]; then
  echo "publish-crates: the crates disagree on the version: ${versions[*]}" >&2
  exit 1
fi
version=${versions[0]}

order=(shadcn-egui-core)
for n in "${members[@]}"; do
  case "$n" in shadcn-egui-font-*|shadcn-egui-icons-*) order+=("$n") ;; esac
done
order+=(shadcn-egui)
if [ "$(printf '%s\n' "${order[@]}" | sort)" != "$(printf '%s\n' "${members[@]}")" ]; then
  echo "publish-crates: the publish order does not list every workspace crate exactly once:" >&2
  diff <(printf '%s\n' "${order[@]}" | sort) <(printf '%s\n' "${members[@]}") >&2 || true
  exit 1
fi

if [ "${PLAN_ONLY:-}" = 1 ]; then
  for n in "${order[@]}"; do echo "$n $version"; done
  exit 0
fi

mkdir -p target
log=target/publish-crates.log
hdr=target/publish-crates.headers

# wait_rate_limit <crate> <epoch|"">: sleeps until the
# given time plus a margin, or 10 min when no time is known.
wait_rate_limit() {
  local n=$1 until=$2 secs note=
  if [ -n "$until" ]; then
    secs=$((until - $(date +%s) + 5))
    [ "$secs" -ge 5 ] || secs=5
  else
    secs=600
    note=" (no retry time given, waiting a fixed 10 min)"
  fi
  echo "publish-crates: rate limit; waiting until $(date -d "@$(($(date +%s) + secs))" '+%F %T') ($(((secs + 59) / 60)) min) before retrying $n$note"
  sleep "$secs"
}

for n in "${order[@]}"; do
  while :; do
    if ! status=$(curl -sS -o /dev/null -D "$hdr" -w '%{http_code}' -A "$ua" "https://crates.io/api/v1/crates/$n/$version"); then
      echo "publish-crates: cannot reach crates.io while checking $n $version" >&2
      exit 1
    fi
    [ "$status" = 429 ] || break
    if [ "${NO_WAIT:-}" = 1 ]; then
      echo "publish-crates: crates.io rate limit (HTTP 429) while checking $n; re-run later" >&2
      exit 2
    fi
    ra=$(sed -n 's/^[Rr]etry-[Aa]fter: *\([0-9][0-9]*\).*/\1/p' "$hdr" | head -1)
    wait_rate_limit "$n" "${ra:+$(($(date +%s) + ra))}"
  done
  case "$status" in
    200) echo "skip $n $version (already on crates.io)"; continue ;;
    404) ;;
    *) echo "publish-crates: crates.io answered HTTP $status for $n $version" >&2; exit 1 ;;
  esac
  if [ "${DRY_RUN:-}" = 1 ]; then
    echo "publish $n $version (dry run)"
    continue
  fi
  while :; do
    echo "publish $n $version"
    if cargo publish -p "$n" --locked --jobs "${JOBS:-4}" 2>&1 | tee "$log"; then
      break
    fi
    grep -q 'status 429 Too Many Requests' "$log" || exit 1
    if [ "${NO_WAIT:-}" = 1 ]; then
      echo "publish-crates: crates.io rate limit (HTTP 429) while publishing $n; re-run this script later, it skips what is already published" >&2
      grep -o 'try again after .*' "$log" >&2 || true
      exit 2
    fi
    when=$(sed -n 's/.*try again after \(.* GMT\).*/\1/p' "$log" | head -1)
    until=
    [ -z "$when" ] || until=$(date -d "$when" +%s 2>/dev/null) || until=
    wait_rate_limit "$n" "$until"
  done
done
if [ "${DRY_RUN:-}" = 1 ]; then echo "dry run: nothing published"; else echo "all published"; fi
