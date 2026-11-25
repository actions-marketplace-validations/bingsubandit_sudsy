#!/usr/bin/env bash
# Test of scripts/clean-slots.sh on a throwaway slot: it deletes Cargo profile directories (`debug`, `incremental`)
# and keeps a `debug` or `incremental` directory of anything else (the npm package `debug` of a parity install).
set -euo pipefail
cd "$(dirname "$0")/.."
slot=$(mktemp -d)
trap 'rm -rf "$slot"' EXIT
mkdir -p "$slot/debug/.fingerprint" "$slot/debug/incremental" "$slot/parity/node_modules/debug" "$slot/parity/node_modules/x/incremental"
bash scripts/clean-slots.sh --all "$slot" >/dev/null
[ ! -e "$slot/debug" ] || { echo "FAIL: the Cargo debug directory is still there" >&2; exit 1; }
[ -d "$slot/parity/node_modules/debug" ] || { echo "FAIL: the npm package debug was deleted" >&2; exit 1; }
mkdir -p "$slot/debug/.fingerprint" "$slot/debug/incremental"
bash scripts/clean-slots.sh "$slot" >/dev/null
[ ! -e "$slot/debug/incremental" ] && [ -d "$slot/debug/.fingerprint" ] || { echo "FAIL: incremental handling" >&2; exit 1; }
[ -d "$slot/parity/node_modules/x/incremental" ] || { echo "FAIL: a foreign incremental directory was deleted" >&2; exit 1; }
echo "clean-slots tests passed"
