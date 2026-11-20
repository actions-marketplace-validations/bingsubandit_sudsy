# Performance checklist

Seven patterns caused every bench overrun so far. Check a change against them before `finish.sh`.

1. **An owned `SyntaxToken` or `SyntaxNode` in a struct.** It stops rowan's cursor reuse (+27% allocations in HCL, +12% in SQL). Hold a position (offset, index, `Kind`) or borrow for the walk.
2. **A `String` per token, row or word.** Borrow `&'static str` from the embedded text (`include_str!`, rows read once) or a slice of the source.
3. **A table or regex built per file.** Build once per process (`LazyLock`, a provider field), not per `format` or `lint` call.
4. **An extra full-tree walk.** Let the existing walk carry the work (a rule hook, a visitor state).
5. **An extra rewrite or reparse round when the first changed nothing.** Skip the next round when the previous one made no edit.
6. **`format!` per item.** Write into one reused buffer, or push parts.
7. **A hot helper left to the inliner.** A lint or format metric of a language that never touches your change moved by 1-2% with the same step in two unrelated branches: a helper on a per-node path (`Grammar::has_tag` was the case) was inlined at some call sites and not at others, and which ones flips with code growth elsewhere. Mark per-node helpers whose argument is a literal `#[inline(always)]` and keep the cold branch out of line.

After CI flags an allocation overrun, measure on your own build, no valgrind and no base build: allocation counts match between arm and x86, so compare with CI's number directly. From the worktree root:

`CARGO_TARGET_DIR=<slot> cargo run --release -q -p perf-bench --bin perf-bench --features count -- fixed <lang> <phase>`

It prints `RESULT <lang> <phase> files=.. bytes=.. work=.. allocs=N` (`<lang>` and `<phase>` as in the bench table; `perf-bench list` shows them). Record hot-path numbers in [numbers.md](numbers.md); the tools are in [tooling.md](tooling.md).
