# Sourced by the scripts that use the build slot, after `cd` to the worktree root: when
# CARGO_TARGET_DIR is unset, it is read from .cargo/tmp/SLOT (one line, the slot's absolute path).
if [ -z "${CARGO_TARGET_DIR:-}" ] && [ -s .cargo/tmp/SLOT ]; then
  CARGO_TARGET_DIR=$(tr -d '[:space:]' <.cargo/tmp/SLOT); export CARGO_TARGET_DIR
fi
