#!/usr/bin/env bash
# npm-groovy-lint over the capped corpus copy (docs/ai/design/gradle.md): lint with its default config (JSON), and a --format run
# on a copy (the formatted files for the format comparison). WORK as in evidence_gradle_tools.sh; the corpus copy is WORK/corpus.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WORK="${1:-$(cat "$ROOT/.cargo/tmp/SLOT")/gradle-eval}"
export JAVA_HOME="$WORK/jdk" PATH="$WORK/jdk/bin:$PATH"
NLG="$WORK/nlg/node_modules/.bin/npm-groovy-lint"
cd "$WORK"
mkdir -p out
echo "lint (default config)"
( time "$NLG" --path "$WORK/corpus" --files '**/*.gradle' --output "$WORK/out/nlg-lint.json" --failon none ) 2>&1 | tail -15
echo "format (--format: the format preset, applied on a copy)"
rm -rf "$WORK/corpus-nlg"; cp -r "$WORK/corpus" "$WORK/corpus-nlg"
( time "$NLG" --path "$WORK/corpus-nlg" --files '**/*.gradle' --format --nolintafter --failon none ) 2>&1 | tail -15
"$NLG" --killserver >/dev/null 2>&1 || true
