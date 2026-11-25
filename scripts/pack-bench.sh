#!/usr/bin/env bash
# Static grammars (every release) vs pack mode (switched off), same commit, release builds with every
# pack: sizes of both binaries (raw and gzip) and of the packs, `--version` and formatting one small
# file (startup, mean of 200 runs), peak memory of both (`/usr/bin/time`), then `lint` of a 4.5 MB
# TOML file, interleaved wall clock (best and median of 8 runs per round).
set -euo pipefail
cd "$(dirname "$0")/.."
. scripts/lib/slot.sh
target=${CARGO_TARGET_DIR:-target}
work=$target/pack-bench
exe=$(case "$(uname -s)" in MINGW* | MSYS* | CYGWIN*) echo .exe ;; esac)
mkdir -p "$work"

scripts/pack.sh "$work"
cargo build --release -p cli
cp "$target/release/sudsy$exe" "$work/static$exe"
SUDSY_PACK_MANIFEST=$(cd "$work" && { pwd -W 2>/dev/null || pwd; })/manifest.txt cargo build --release -p cli --no-default-features --features pack-mode
cp "$target/release/sudsy$exe" "$work/packed$exe"
export SUDSY_CACHE_DIR=$work/cache
"$work/packed$exe" languages add --all --from "$work"

for b in static packed; do
  echo "$b: $(wc -c <"$work/$b$exe") bytes, $(gzip -9 -c "$work/$b$exe" | wc -c) bytes gzip"
done
echo "packs: $(cat "$work"/sudsy-*-"$(rustc -vV | sed -n 's/^host: //p')".* 2>/dev/null | wc -c) bytes (all, raw)"

us() { local s e; s=$(date +%s%N); "$@" >/dev/null 2>&1; e=$(date +%s%N); echo $(((e - s) / 1000))  ; }
mkdir -p "$work/small"
printf '[package]\nname   = "x"\nversion="1"\n' >"$work/small/a.toml"
for round in 1 2 3; do
  for b in static packed; do
    for what in version format; do
      sum=0
      for _ in $(seq 200); do
        if [ $what = version ]; then t=$(us "$work/$b$exe" --version); else t=$(us "$work/$b$exe" format "$work/small/a.toml"); fi
        sum=$((sum + t))
      done
      echo "$b round $round startup $what: mean $((sum / 200)) us"
    done
  done
done
for b in static packed; do
  /usr/bin/time -f "$b peak memory formatting one small file: %M KB" "$work/$b$exe" format "$work/small/a.toml"
done

for i in $(seq 0 59999); do
  printf '[t%d]\nname = "item %d"\nvalues = [1, 2, 3, %d]\nenabled = true\n\n' "$i" "$i" "$i"
done >"$work/big.toml"
ms() { local s e; s=$(date +%s%N); "$@" >/dev/null 2>&1; e=$(date +%s%N); echo $(((e - s) / 1000000)); }
for round in 1 2 3; do
  for b in static packed; do
    t=()
    for _ in 1 2 3 4 5 6 7 8; do t+=("$(ms "$work/$b$exe" lint --analysis fast "$work/big.toml")"); done
    sorted=($(printf '%s\n' "${t[@]}" | sort -n))
    echo "$b round $round: best ${sorted[0]} ms, median ${sorted[4]} ms"
  done
done
