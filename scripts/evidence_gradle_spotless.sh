#!/usr/bin/env bash
# Spotless groovyGradle with greclipse (the Eclipse Groovy formatter, default settings) over a copy of the capped corpus
# (docs/ai/design/gradle.md). The formatted tree stays in WORK/spotless/corpus; a file the step cannot parse stays as it was
# (`ignoreErrorForStep`). WORK as in evidence_gradle_tools.sh.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WORK="${1:-$(cat "$ROOT/.cargo/tmp/SLOT")/gradle-eval}"
export JAVA_HOME="$WORK/jdk" PATH="$WORK/jdk/bin:$PATH" GRADLE_USER_HOME="$WORK/gradle-home"
cd "$WORK/spotless"
rm -rf corpus; cp -r "$WORK/corpus" corpus
( time "$WORK/gradle/bin/gradle" --no-daemon --console=plain spotlessApply ) > "$WORK/out/spotless.log" 2>&1; tail -5 "$WORK/out/spotless.log"; echo "files the step could not format: $(grep -c "^Step .eclipse" "$WORK/out/spotless.log")"
