# Sourced by gate.sh and parity.sh. `fail_summary LOG`: what failed in a step's log, at most 10 lines
# per kind and nothing else (`fail_summary LOG [SHOWN_LOG_PATH]`): rustc and clippy errors as `file:line  message`, failed test names
# (nextest, cargo test, unittest), panic locations, parity verdicts. No match: the last 15 lines.
fail_summary() {
  local out
  out=$(sed -E 's/\x1b\[[0-9;]*m//g' "$1" | awk '
    function emit(k, s) { if (++c[k] <= 10) { print "  " s; any = 1 } else if (c[k] == 11) print "  (" k ": more in the log)" }
    /^error(\[[A-Za-z0-9]+\])?: / { msg = $0; sub(/^error(\[[A-Za-z0-9]+\])?: /, "", msg)
      if (msg ~ /^(could not compile|aborting|test failed)/ || msg ~ /^[0-9]+ tests? failed/) msg = ""; next }
    /^ *--> / && msg != "" { emit("error", $2 "  " msg); msg = ""; next }
    /^ +(FAIL|TIMEOUT|SIGABRT|SIGSEGV) \[/ { s = $0; sub(/^ +[A-Z]+ \[[^]]*\] +(\([0-9]+\/[0-9]+\) +)?/, "", s); if (!(s in seen)) { seen[s] = 1; emit("test", s) } next }
    /^test .* \.\.\. FAILED$/ { emit("test", $2); next }
    /^Diff in / { s = $0; sub(/^Diff in /, "", s); sub(ENVIRON["PWD"] "/", "", s); sub(/:$/, "", s); emit("fmt", s); next }
    /^(FAIL|ERROR): / { emit("test", $0); next }
    / panicked at / { s = $0; sub(/.* panicked at /, "", s); sub(/:$/, "", s); emit("panic", s); next }
    /^(Source|Snapshot file): / { emit("snapshot", $0); next }
    /^[^ ]+: (DIFFERENT|REVIEW|changes on a second format|refused by the safety)|^MISS |MISMATCH|^  (bug|gap [^:]*): |ratchet|^(STALE|UNDEFINED|UNUSED)|^  only (sudsy|[a-z]+):/ { emit("parity", $0); next }
    /mismatch\(es\)|^target / { emit("totals", $0); next }
    /^(Traceback|[A-Za-z]*Error:)/ { emit("python", $0) }
    END { exit any ? 0 : 1 }') || out=$(tail -n 15 "$1")
  printf '%s\n' "$out"
  echo "full log: ${2:-$1}"
  # CI has no log file to read: the end of it goes with the summary.
  [ -z "${GITHUB_ACTIONS:-}" ] || { echo "-- end of the log"; tail -n 30 "$1"; }
}
