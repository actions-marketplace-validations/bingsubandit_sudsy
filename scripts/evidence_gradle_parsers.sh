#!/usr/bin/env bash
# The parser comparison of docs/ai/design/gradle.md: every tree-sitter candidate of evidence_gradle_tools.sh and the hand-written
# lexer prototype, over the capped corpus copy WORK/corpus (list: WORK/files.txt). One build per candidate in WORK/target-<name>.
# Prints the numbers of the table; per-file results go to WORK/out/ts-<name>.tsv.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WORK="${1:-$(cat "$ROOT/.cargo/tmp/SLOT")/gradle-eval}"
MANIFEST="$ROOT/scripts/evidence_gradle_parsers/Cargo.toml"
find "$WORK/corpus" -name '*.gradle' | sort > "$WORK/files.txt"
mkdir -p "$WORK/out"
echo "machine: $(lscpu | sed -n 's/^Model name: *//p'), $(nproc) cores, $(rustc --version)"

CARGO_TARGET_DIR="$WORK/target-lex" cargo build --release --manifest-path "$MANIFEST" 2>&1 | tail -1
BASE="$WORK/target-lex/release/gradle-parsers"
cp "$BASE" "$WORK/out/base"; strip "$WORK/out/base"; BASE_SIZE=$(stat -c %s "$WORK/out/base")
echo "== hand-written lexer prototype (lexer only: the parser is not written)"
"$BASE" lex "$WORK/files.txt" | tail -4
"$BASE" lexspeed "$WORK/files.txt"

for d in "$WORK"/ts/*/; do
  n="$(basename "$d")"; src="$d/src"
  [ -f "$src/parser.c" ] || { echo "== $n: no generated parser.c in the repository ($(cat "$d/PIN"))"; continue; }
  sym="$(grep -o 'tree_sitter_[a-zA-Z0-9_]*(void)' "$src/parser.c" | grep -v external_scanner | head -1 | sed 's/(void)//')"
  echo "== $n ($(cat "$d/PIN")), symbol $sym, parser.c $(stat -c %s "$src/parser.c") bytes, scanner: $(ls "$src" | grep -c '^scanner' || true)"
  GRAMMAR_SRC="$src" GRAMMAR_SYMBOL="$sym" CARGO_TARGET_DIR="$WORK/target-$n" cargo build --release --manifest-path "$MANIFEST" 2>&1 | grep -E '^error' || true
  B="$WORK/target-$n/release/gradle-parsers"
  cp "$B" "$WORK/out/bin-$n"; strip "$WORK/out/bin-$n"
  echo "stripped binary +$(( ($(stat -c %s "$WORK/out/bin-$n") - BASE_SIZE) / 1024 )) KB over the lexer-only binary"
  "$B" parse "$WORK/files.txt" > "$WORK/out/ts-$n.tsv"
  python3 -I - "$WORK/out/ts-$n.tsv" <<'EOF'
import sys, collections
rows = [l.rstrip("\n").split("\t") for l in open(sys.argv[1], newline="\n", errors="replace")]
ok = sum(1 for r in rows if r[2] == "0" and r[3] == "0")
print(f"files {len(rows)}; without error or missing node {ok} ({100*ok/len(rows):.1f}%); error nodes {sum(int(r[2]) for r in rows)}, missing {sum(int(r[3]) for r in rows)}; leaves and blank gaps tile the source: {sum(r[6]=='true' for r in rows)}")
c = collections.Counter(r[5].split(" ")[0][:24] if len(r) > 5 else "" for r in rows if r[2] != "0" or r[3] != "0")
print("first error starts with:", c.most_common(12))
EOF
  echo "error-free files whose leaves and blank gaps do not tile: $("$B" gaps "$WORK/files.txt" | grep -c 'errors false' || true); first gaps:"; "$B" gaps "$WORK/files.txt" | grep 'errors false' | cut -f2 | sort | uniq -c | sort -rn | head -4 || true
  "$B" speed "$WORK/files.txt"
  "$B" mutants "$WORK/files.txt" 250
  for k in paren brace; do for n_ in 1000 10000 100000; do "$B" deep $k $n_ 2>&1 | tail -1 || echo "deep $k $n_: crashed"; done; done
  "$B" probe "$ROOT/scripts/evidence_gradle_parsers/probes.groovy" > "$WORK/out/probe-$n.tsv"
  echo "probes failing: $(grep -c ERR "$WORK/out/probe-$n.tsv") of $(wc -l < "$WORK/out/probe-$n.tsv")"
done
