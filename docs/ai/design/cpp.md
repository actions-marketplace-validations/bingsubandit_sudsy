# C and C++: design (deleted when C/C++ is usable)

Why `lang-cpp` is a token-level structure and not a parser.

## Parsers compared
On the pinned corpus (Linux kernel, SQLite, LLVM, folly, fmt, range-v3; 6,594 files), without macro definitions or include paths:

| Parser | Files with errors |
|---|---|
| tree-sitter-c / tree-sitter-cpp | 20-67% per project |
| `cpp_parser` (Rust) | 5-38% per project |
| oak-c / oak-cpp | need nightly Rust, excluded |

The failures come from unknown macros (`FMT_BEGIN_NAMESPACE`, `DEFINE_MUTEX(x)`, `__init`), not from the preprocessor directives. Full parsing needs the headers (libclang), which is a worker and a build configuration.

## Decision
clang-format's own model: lex once, group tokens into unwrapped lines and bracket groups, treat macros by configuration (`ForEachMacros`, `StatementMacros`, ...). The lexer is ours and lossless (350 MB/s alone, 56-69 MB/s with the tree). The tree never fails, so the formatter can refuse nothing and a later lint layer can still match on lines.

## Preprocessor
clang-format parses each `#if` branch configuration separately and merges the replacements (its `lib/Format` is about 29K lines). Sudsy parses once: at `#else`/`#elif` the groups opened inside the branch are closed back to the depth at `#if`; at `#endif` the last branch's state continues. Branch-asymmetric groups (a branch opens or closes a bracket the others do not) occur in 0.2-6.5% of files; there `PpGroup` is omitted and only `Directive` nodes remain (`PpGroup` is 651 of 823 `#if` in llvm, 856/892 fmt, 3841/4211 sqlite, 2017/2355 folly, 573/930 range-v3, 7939/8427 linux).
