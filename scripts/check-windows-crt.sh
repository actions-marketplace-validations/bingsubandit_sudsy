#!/usr/bin/env bash
# Fails when a Windows binary imports the Visual C++ runtime, which a clean Windows lacks
# (`+crt-static` in .cargo/config.toml links it in). Usage: check-windows-crt.sh <exe or dll>...
set -euo pipefail
bad=0
for f in "$@"; do
  wc -c "$f"
  if grep -aqi 'vcruntime140[_0-9]*\.dll' "$f"; then echo "$f imports vcruntime140.dll" >&2; bad=1; fi
done
exit $bad
