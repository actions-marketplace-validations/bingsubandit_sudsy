#!/usr/bin/env bash
# Wall time of `sudsy lint` over a folder with the default rules and with a preset's rules on.
#   scripts/preset-lint-time.sh PRESET FOLDER [SUDSY]    e.g. batch/blinter bench/inputs/bat
# Best of 5 runs each, in a scratch copy of FOLDER with a sudsy.yaml; SUDSY defaults to the optimized dev build of the slot.
set -euo pipefail
preset=$1 folder=$2
sudsy=${3:-${CARGO_TARGET_DIR:-$(cat "$(dirname "$0")/../.cargo/tmp/SLOT")}/debug/sudsy}
work=$(mktemp -d "${TMPDIR:-/scratch/tmp}/preset-lint.XXXXXX")
trap 'rm -rf "$work"' EXIT
cp -r "$folder"/. "$work"/
best() {
  local b=999999 t s
  for _ in 1 2 3 4 5; do
    s=$(date +%s%N)
    (cd "$work" && "$sudsy" lint --format json . >/dev/null) || true
    t=$((($(date +%s%N) - s) / 1000000))
    ((t < b)) && b=$t
  done
  echo "$b"
}
rm -f "$work/sudsy.yaml"
echo "files: $(find "$work" -type f | wc -l), bytes: $(cat $(find "$work" -type f) | wc -c)"
echo "default rules: $(best) ms"
printf 'extends:\n  - preset:%s\n' "$preset" >"$work/sudsy.yaml"
echo "preset $preset: $(best) ms"
