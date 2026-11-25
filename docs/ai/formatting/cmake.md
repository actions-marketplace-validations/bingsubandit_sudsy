# CMake (`lang-cmake`)

Plan and phases: [../design/cmake.md](../design/cmake.md).

## Parser
Hand-written lexer and parser (`crates/lang-cmake/src/lexer.rs`, `parser.rs`) on the kit `core_syntax::parse` (`Lexer`, `Parser`, trivia attachment, depth guard); `Quoted` is scanned by the `string` row of the grammar (`StringSyntax::scan`). `grammars/cmake.grammar` has the tokens (`Unquoted`, `Quoted`, `Bracket`, `LParen`, `RParen`, line and bracket comments) and the nodes `File`, `Command` (name, `(`, `ArgumentList`, `)`), `ArgumentList`, `Argument`, `ParenGroup`. Blocks (`if` ... `endif`) are not nodes: they are pairs of commands, and the pairing is a later step. Variable references and generator expressions are text inside an `Unquoted` or `Quoted` token: no rule reads their parts yet.

- A word is an `Unquoted` run up to a blank, a parenthesis, `#` or `"`; `\` escapes the next character, a line break too. A `"` starts a `Quoted` token (`\` newline continues it, a bare newline is part of it) and `[`, `=`s, `[` at the start of a word a `Bracket` token (any number of `=`); `#[=[ ... ]=]` is a bracket comment, any other `#` a line comment.
- Words that touch are the parts of one `Argument`: `-DFOO="bar"` is `Unquoted` `Quoted`, `"a"b` is `Quoted` `Unquoted`, `$<$<CONFIG:Debug>:dbg;"x y">` is three parts. A `(` inside a word is a `ParenGroup` of its own, so `$<$<X:GNU>:(a b)>` is an `Argument`, a `ParenGroup` and an `Argument`; no blank between them is what the layout reads.
- A leading BOM is whitespace; `\r\n`, `\n` and a lone `\r` end a line.

Errors, none of them stops the parse and every token stays in the tree:
- Tokens that are no command (`@VAR@` alone on a line, a stray `)`) are one `ERROR` node up to the next `name(`.
- An unclosed `(` ends its command at the next command that starts at column 0 (name and `(`), else at the end of the file; the closing parentheses are paired in one pass over the tokens before the parse, so a balanced command never stops early. An unterminated quoted text, bracket text or bracket comment ends the same way. A deleted line damages one command: of 600 sampled deletions, 599 lose at most two commands.
- Parentheses nested deeper than 128 levels are one `ERROR` node.

Files: `CMakeLists.txt`, `*.cmake` except `*.<ext>.cmake` for the extensions of `exclude-names` in `lang.yaml` (`config.h.cmake`, `foo.pc.cmake`: templates of other languages; `arm.toolchain.cmake` is CMake). The corpus has two more templates that match by name, `run_nvcc.cmake` (`@VAR@` lines) and `clang-format.cmake` (YAML).

## Formatting
Not formatted yet: the file is kept as written (`core_format::VerbatimFormatter`).

## Conformance
`tests/conformance/fetch-cmake-corpus.sh` fetches the `CMakeLists.txt` and `*.cmake` files of LLVM, CMake, Qt base, VTK, OpenCV, extra-cmake-modules and protobuf at pinned commits (9,066 files, 22.8 MB; the measurement of the plan left `.git*` folders out): every file prints back byte for byte and fits the grammar, 31 have parse errors and all are templates (`cargo test --profile corpus -p lang-cmake --test integration -- --ignored --nocapture conformance`, 5 s). Speed and the line-deletion test: [../performance/numbers.md](../performance/numbers.md).

## Options
<!-- generated:options -->
| Option | Values | Default | Description |
|---|---|---|---|
| `indent_size` | int 1-16 | `4` | Columns per indentation level (also the width of a tab). |
| `line_width` | int 20-1000 | `80` | Preferred maximum line width in columns. |
| `indent_style` | `space`, `tab` | `space` | Indent with spaces or tabs. |
| `final_newline` | bool | `true` | End the file with a newline. |
| `end_of_line` | `auto`, `lf`, `crlf` | `auto` | Line break style of the output: `lf`, `crlf`, or `auto` to keep the file's first line break. |
<!-- /generated -->
