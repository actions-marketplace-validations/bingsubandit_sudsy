#!/usr/bin/env bash
# The pack-mode check (the `pack-mode` CI job; no release is built this way): clippy and tests of the
# crates behind the `pack-mode` feature (the installed set, `sudsy languages`, the LSP install
# prompts), then a real pack-mode CLI (no grammars linked in, the hashes of a toml pack embedded)
# that installs the pack from a directory and formats a toml file.
set -euo pipefail
cd "$(dirname "$0")/.."
. scripts/lib/slot.sh
target=$(realpath -m "${CARGO_TARGET_DIR:-target}")
features=cli/pack-mode,lsp/pack-mode
crates=(-p core-packs -p languages -p lsp -p cli)

cargo clippy "${crates[@]}" --all-targets --features "$features" -- -D warnings
cargo test "${crates[@]}" --features "$features"

work=$target/pack-mode
rm -rf "$work"
mkdir -p "$work/proj"
scripts/pack.sh "$work" toml
SUDSY_PACK_MANIFEST=$(cd "$work" && { pwd -W 2>/dev/null || pwd; })/manifest.txt \
  cargo build --release -p cli --no-default-features --features pack-mode
exe=$target/release/sudsy
export SUDSY_CACHE_DIR=$work/cache
printf 'a   = 1\n' >"$work/proj/a.toml"
cd "$work/proj"
rc=0
"$exe" format a.toml 2>/dev/null || rc=$?
[ "$rc" -eq 3 ] || { echo "a missing pack must exit 3, got $rc" >&2; exit 1; }
"$exe" languages add toml --from "$work"
"$exe" format a.toml
[ "$(cat a.toml)" = "a = 1" ] || { echo "toml not formatted by the pack" >&2; exit 1; }
