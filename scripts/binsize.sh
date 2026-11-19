#!/usr/bin/env bash
# Stripped release CLI size per revision: scripts/binsize.sh [REV...] (default: HEAD).
# Each REV is built in a temporary worktree under <slot>/binsize-wt with `cargo build --release -p cli`
# (default release profile, features of the checkout) into <slot>/binsize-target, copied and `strip`ped.
set -euo pipefail
cd "$(dirname "$0")/.."
. scripts/lib/slot.sh
slot=${CARGO_TARGET_DIR:-target}
export CARGO_TARGET_DIR=$slot/binsize-target
[ $# -gt 0 ] || set -- HEAD
for rev in "$@"; do
  sha=$(git rev-parse --short "$rev")
  wt=$slot/binsize-wt/$sha
  [ -d "$wt" ] || git worktree add --detach "$wt" "$sha" >/dev/null
  (cd "$wt" && cargo build --release -p cli -q)
  cp "$CARGO_TARGET_DIR/release/sudsy" "$slot/binsize-$sha"
  strip "$slot/binsize-$sha"
  printf '%s %d\n' "$sha" "$(stat -c %s "$slot/binsize-$sha")"
  git worktree remove --force "$wt"
done
