#!/usr/bin/env bash
# Wall time and peak memory of Sudsy against ruff, Biome, oxlint and Prettier on real projects, check mode, every run a new
# process (docs/ai/performance/numbers.md, "Speed against ruff, Biome and oxlint"):
#   Python: Django (corpus/python/django/django: 883 files) format and lint against ruff (`--isolated`, default rules);
#   JavaScript: three.js src and express lib+test, 8 copies, format against Biome and Prettier, lint (default rules)
#   against oxlint and Biome.
# Prints one markdown row per task and tool: best of $runs wall seconds, peak RSS in MB of that run.
# Needs: fetch-python-corpus.sh and fetch-js-corpus.sh run, cargo build --release -p cli, node/npm, curl, /usr/bin/time.
# Pinned: ruff $ruff (release binary), Biome, oxlint and Prettier in speed-tools/package.json (with its lock). Usage: speed-lint-format.sh [runs]
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
repo="$here/../.."
runs="${1:-5}"
ruff=0.17.4
sudsy="${CARGO_TARGET_DIR:-$repo/target}/release/sudsy"
work="${SUDSY_SPEED_DIR:-${TMPDIR:-/tmp}/sudsy-speed-lint-format}"
tools="$here/speed-tools"
bin="$tools/node_modules/.bin"

cargo build --release -q -p cli --manifest-path "$repo/Cargo.toml" || exit 1
mkdir -p "$work/tools"
(cd "$tools" && npm install --silent --no-audit --no-fund) || exit 1
if [ ! -x "$work/tools/ruff" ]; then
  tar="ruff-$(uname -m)-unknown-linux-gnu"
  curl -fsSL "https://github.com/astral-sh/ruff/releases/download/$ruff/$tar.tar.gz" | tar -xz -C "$work/tools" --strip-components=1 "$tar/ruff" || exit 1
fi

rm -rf "$work/py" "$work/js"; mkdir -p "$work/py" "$work/js"
cp -r "$here/corpus/python/django/django" "$work/py/django"
for i in 1 2 3 4 5 6 7 8; do
  mkdir -p "$work/js/c$i/three" "$work/js/c$i/express"
  cp "$here"/corpus/js/three.js/src/*.js "$work/js/c$i/three/"
  cp -r "$here/corpus/js/express/lib" "$here/corpus/js/express/test" "$work/js/c$i/express/"
done

# bench TASK TOOL DIR CMD... : best wall seconds of $runs runs in $DIR, and that run's peak RSS
bench() {
  local task="$1" tool="$2" dir="$3"; shift 3
  local best="" mem="" t m
  for _ in $(seq "$runs"); do
    /usr/bin/time -f '%e %M' -o "$work/time" bash -c 'cd "$0" && "$@" >/dev/null 2>&1' "$dir" "$@"
    read -r t m < <(tail -n1 "$work/time")
    if [ -z "$best" ] || awk "BEGIN{exit !($t < $best)}"; then best=$t mem=$m; fi
  done
  printf '| %s | %s | %.2f s | %d MB |\n' "$task" "$tool" "$best" "$((mem / 1024))"
}

printf 'ruff %s, %s, %s, %s; %s files, %s MB (Python), %s files, %s MB (JavaScript); %s runs\n' "$ruff" \
  "$("$bin/biome" --version)" "$("$bin/oxlint" --version)" "prettier $("$bin/prettier" --version)" \
  "$(find "$work/py" -name '*.py' | wc -l)" "$(du -sm "$work/py" | cut -f1)" \
  "$(find "$work/js" -name '*.js' | wc -l)" "$(du -sm "$work/js" | cut -f1)" "$runs"
echo '| Task | Tool | Wall | Peak memory |'
echo '|---|---|---|---|'
py="$work/py"; js="$work/js"
bench "Django format" sudsy "$py" "$sudsy" format --check .
bench "Django format" ruff "$py" "$work/tools/ruff" format --check --isolated --no-cache .
bench "Django lint" sudsy "$py" "$sudsy" lint .
bench "Django lint" ruff "$py" "$work/tools/ruff" check --isolated --no-cache .
bench "JS format" sudsy "$js" "$sudsy" format --check .
bench "JS format" biome "$js" "$bin/biome" format --colors=off --max-diagnostics=0 .
bench "JS format" prettier "$js" "$bin/prettier" --check --no-cache --log-level warn .
bench "JS lint" sudsy "$js" "$sudsy" lint .
bench "JS lint" oxlint "$js" "$bin/oxlint" .
bench "JS lint" biome "$js" "$bin/biome" lint --colors=off --max-diagnostics=0 .
