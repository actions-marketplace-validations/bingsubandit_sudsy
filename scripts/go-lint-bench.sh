#!/usr/bin/env bash
# Wall time of `sudsy lint` over a folder of the Go corpus with one rule on at a time (best of 7).
# Usage: scripts/go-lint-bench.sh [DIR] [RULE...]   (needs release/sudsy in the build slot and the Go corpus)
set -euo pipefail
cd "$(dirname "$0")/.."
slot=${CARGO_TARGET_DIR:-$(cat .cargo/tmp/SLOT)}
exe=$slot/release/sudsy
dir=$(realpath "${1:-tests/conformance/corpus/go/go/src/net}")
shift || true
rules=${*:-"none missing-doc-comment doc-first-word missing-package-comment unused-parameter redefines-builtin-id var-naming"}
work=$(mktemp -d "${TMPDIR:-/tmp}/golintbench.XXXXXX")
trap 'rm -rf "$work"' EXIT
cp -r "$dir" "$work/src"
files=$(find "$work/src" -name '*.go' | wc -l)
echo "files: $files ($dir)"
for rule in $rules; do
  if [ "$rule" = none ]; then
    printf 'lint:\n  rules: {}\n' >"$work/sudsy.yaml"
  else
    printf 'lint:\n  rules:\n    %s: warn\n' "$rule" >"$work/sudsy.yaml"
  fi
  best=999999
  for _ in 1 2 3 4 5 6 7; do
    start=$(date +%s%N)
    (cd "$work" && "$exe" lint src) >/dev/null 2>&1 || true
    ms=$((($(date +%s%N) - start) / 1000000))
    [ "$ms" -lt "$best" ] && best=$ms
  done
  echo "$rule: $best ms"
done
