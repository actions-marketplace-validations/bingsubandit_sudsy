# Performance checklist

Six patterns caused every bench overrun so far. Check a change against them before `finish.sh`.

1. **An owned `SyntaxToken` or `SyntaxNode` in a struct.** It stops rowan's cursor reuse (+27% allocations in HCL, +12% in SQL). Hold a position (offset, index, `Kind`) or borrow for the walk.
2. **A `String` per token, row or word.** Borrow `&'static str` from the embedded text (`include_str!`, rows read once) or a slice of the source.
3. **A table or regex built per file.** Build once per process (`LazyLock`, a provider field), not per `format` or `lint` call.
4. **An extra full-tree walk.** Let the existing walk carry the work (a rule hook, a visitor state).
5. **An extra rewrite or reparse round when the first changed nothing.** Skip the next round when the previous one made no edit.
6. **`format!` per item.** Write into one reused buffer, or push parts.

After CI flags an allocation overrun, measure on your own build, no valgrind and no base build: allocation counts match between arm and x86, so compare with CI's number directly. From the worktree root:

`CARGO_TARGET_DIR=<slot> cargo run --release -q -p perf-bench --bin perf-bench --features count -- fixed <lang> <phase>`

It prints `RESULT <lang> <phase> files=.. bytes=.. work=.. allocs=N` (`<lang>` and `<phase>` as in the bench table; `perf-bench list` shows them). Record hot-path numbers in [numbers.md](numbers.md); the tools are in [tooling.md](tooling.md).
