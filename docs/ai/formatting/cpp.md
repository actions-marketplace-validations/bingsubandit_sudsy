# C and C++

Two languages, `c` and `cpp`, one crate (`lang-cpp`) with one hand-written lexer and one structural layer. There is no parser for the full grammar: C and C++ cannot be parsed without the preprocessor and the types the headers declare, and the tree-sitter grammars and `cpp_parser` fail on 5-67% of real files (unknown macros). The tree is a token-level structure in the manner of clang-format's unwrapped lines, and it never fails.

Files: `.c` and `.h` are `c`; `.cc .cpp .cxx .hpp .hh .hxx .ipp` are `cpp`. A `.h` is C because the extension does not say, and the lexer differs only in keywords (C++ keywords are identifiers in C); C++ projects that use `.h` select `cpp` with a `files` override.

## Lexer (`lexer.rs`)
Lossless tokens: identifiers, keywords (by dialect), numbers (digit separators, hex floats, suffixes), strings and chars with encoding prefixes and raw strings, header names after `#include`/`#include_next`/`#import`, maximal-munch punctuators, and `Other` for a stray byte. Comments and backslash-newline continuations are trivia. A `#` that starts a line begins a directive; the directive's tokens carry its id. The only errors are an unterminated block comment or raw string.

## Structure (`structure.rs`)
One pass over the tokens builds nodes over them:
- `Line`: an unwrapped line (a statement, a declaration, a control-statement header, a `case`/label, a `public:`, a macro invocation alone on its line). A line that ends in a block holds the `BraceGroup`.
- `BraceGroup`, `ParenGroup`, `BracketGroup`: bracket groups, nested in the line that opens them.
- `Directive`: a preprocessor directive including its continuation lines; directives are siblings of lines, never inside one.
- `PpGroup` of `PpBranch`: an `#if`...`#endif` whose branches start and end at the same bracket depth. When a branch opens or closes a bracket the others do not, only the `Directive` nodes remain.

`#if` branches are not each parsed as clang-format does: at `#else`/`#elif` the groups opened in the branch are closed back to the depth at `#if`, and at `#endif` the state of the last branch continues (single-pass bracket reset). 0.2-6.5% of corpus files have branch-asymmetric groups.

## Formatter
Verbatim for now: the text is printed back unchanged. The clang-format-compatible layout is the next step.

## Options
<!-- generated:options -->
| Option | Values | Default | Description |
|---|---|---|---|
| `line_width` | int 20-1000 | `80` | Preferred maximum line width in columns. |
| `indent_size` | int 1-16 | `2` | Columns per indentation level (also the width of a tab). |
| `indent_style` | `space`, `tab` | `space` | Indent with spaces or tabs. |
| `final_newline` | bool | `true` | End the file with a newline. |
| `end_of_line` | `auto`, `lf`, `crlf` | `auto` | Line break style of the output: `lf`, `crlf`, or `auto` to keep the file's first line break. |
<!-- /generated -->

## Conformance
The pinned projects of `tests/conformance/fetch-cpp-corpus.sh` (Linux kernel, SQLite, LLVM, folly, fmt, range-v3; 6,594 files): every file parses without errors, prints back unchanged and passes the shared checks. `cargo test --profile corpus -p lang-cpp --test integration -- --ignored --nocapture conformance` after `bash tests/conformance/fetch-cpp-corpus.sh`; `structure_stats` in the same way prints speed and the shape of the structure per project.
