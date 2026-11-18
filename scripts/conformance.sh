#!/usr/bin/env bash
# Runs the ignored conformance tests of one language crate on its pinned corpus: fetches the corpus
# (tests/conformance/fetch-<language>-corpus.sh), then `cargo test --profile corpus -p <crate>
# -- --ignored` without the measurements. Usage: scripts/conformance.sh lang-<language>
# (docs/ai/development/parity.md, "Nightly").
set -euo pipefail
cd "$(dirname "$0")/.."
crate=${1:?usage: scripts/conformance.sh lang-<language>}
lang=${crate#lang-}
case $lang in
  csharp) fetches=(fetch-corpus.sh csharpier-baseline.sh) ;;
  html) fetches=(fetch-html-corpus.sh fetch-angular-corpus.sh) ;;
  *) fetches=("fetch-$lang-corpus.sh") ;;
esac
for f in "${fetches[@]}"; do
  if [ -f "tests/conformance/$f" ]; then bash "tests/conformance/$f"; fi
done
exec cargo test --profile corpus -p "$crate" -- --ignored --nocapture \
  --skip measure_startup_cost --skip scaling --skip model_cost
