# Sourced by the scripts that use the build slot, after `cd` to the worktree root. The worktree's
# .cargo/tmp/SLOT (one line, the slot's absolute path) wins over CARGO_TARGET_DIR: subagents inherit
# the lead's variable, which names another slot. Without the file, CARGO_TARGET_DIR is used as set.
if [ -s .cargo/tmp/SLOT ]; then
  CARGO_TARGET_DIR=$(tr -d '[:space:]' <.cargo/tmp/SLOT); export CARGO_TARGET_DIR
fi
