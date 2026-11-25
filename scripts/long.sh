#!/usr/bin/env bash
# Runs a long step (gate, bench, parity, cargo) without letting an agent's 5-minute prompt cache expire:
#   long.sh --start CMD [args]   starts it detached (one at a time per worktree), then waits as --wait does
#   long.sh --wait               returns within 4 minutes: the last 40 lines and the step's exit status,
#                                or `LONG RUNNING` with exit 3 (call --wait again)
# Both calls need a Bash `timeout` of 600000. An interrupted --start leaves the step running: --wait picks it up.
# The whole output is in .cargo/tmp/long.out.
set -uo pipefail
cd "$(dirname "$0")/.."
d=.cargo/tmp
case "${1:-}" in
  --start)
    shift
    if [ -f $d/long.pid ] && kill -0 "$(cat $d/long.pid)" 2>/dev/null; then echo "a step is already running" >&2; exit 2; fi
    . scripts/lib/slot.sh
    rm -f $d/long.status
    setsid bash -c 'd=$1; shift; "$@" >"$d/long.out" 2>&1; echo $? >"$d/long.status"' long "$d" "$@" </dev/null >/dev/null 2>&1 &
    echo $! >$d/long.pid ;;
  --wait)
    [ -f $d/long.pid ] || { echo "nothing started" >&2; exit 2; } ;;
  *) sed -n '2,7p' "$0" >&2; exit 2 ;;
esac
for _ in $(seq 1 120); do
  [ -f $d/long.status ] && break
  kill -0 "$(cat $d/long.pid)" 2>/dev/null || { sleep 1; break; }
  sleep 2
done
if [ ! -f $d/long.status ]; then
  if kill -0 "$(cat $d/long.pid)" 2>/dev/null; then echo "LONG RUNNING: call scripts/long.sh --wait again"; exit 3; fi
  echo "LONG DIED without an exit status"; tail -n 40 $d/long.out; exit 1
fi
tail -n 40 $d/long.out; s=$(cat $d/long.status); echo "LONG EXIT $s"; exit "$s"
