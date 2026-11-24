#!/usr/bin/env bash
# Median wall time (ms) of 7 cold `sudsy lint .` runs on a copy of DIR with PRESET (default
# csharp/stylecop), once as it is and once with the named rules off: the cost of those rules.
# Needs the dogfood build (scripts/dogfood.sh). Usage: rule-time.sh DIR RULE [RULE...]
set -uo pipefail
cd "$(dirname "$0")/.."
. scripts/lib/slot.sh
bin=${CARGO_TARGET_DIR:-$PWD/target}/dogfood
export SUDSY_CSHARP_WORKER=$bin/worker/Sudsy.Worker.dll
dir=$(realpath "$1")
shift
work=$(mktemp -d "${TMPDIR:-/tmp}/rule-time.XXXXXX")
cp -r "$dir"/. "$work/"
median() { sort -n | awk '{a[NR]=$1} END{print a[int((NR+1)/2)]}'; }
for mode in on off; do
  {
    echo "extends: [preset:${PRESET:-csharp/stylecop}]"
    if [ $mode = off ]; then
      printf 'csharp:\n  lint:\n    rules:\n'
      for r in "$@"; do echo "      $r: off"; done
    fi
  } > "$work/sudsy.yaml"
  for _ in 1 2 3 4 5 6 7; do
    s=$(date +%s%N)
    (cd "$work" && "$bin/sudsy" lint . > /dev/null 2>&1)
    e=$(date +%s%N)
    echo $(((e - s) / 1000000))
  done | median | sed "s/^/rules $mode: median ms /"
done
rm -rf "$work"
