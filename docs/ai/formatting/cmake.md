# CMake (`lang-cmake`)

Plan and phases: [../design/cmake.md](../design/cmake.md).

## Parser
Hand-written lexer and parser (`crates/lang-cmake/src/lexer.rs`, `parser.rs`) on the kit `core_syntax::parse` (`Lexer`, `Parser`, trivia attachment, depth guard); `Quoted` is scanned by the `string` row of the grammar (`StringSyntax::scan`). `grammars/cmake.grammar` has the tokens (`Unquoted`, `Quoted`, `Bracket`, `LParen`, `RParen`, line and bracket comments) and the nodes `File`, `Command` (name, `(`, `ArgumentList`, `)`), `ArgumentList`, `Argument`, `ParenGroup`. Blocks (`if` ... `endif`) are not nodes: they are pairs of commands, paired by name in the parse (below). Variable references and generator expressions are text inside an `Unquoted` or `Quoted` token: no rule reads their parts yet.

- A word is an `Unquoted` run up to a blank, a parenthesis, `#` or `"`; `\` escapes the next character, a line break too. A `"` starts a `Quoted` token (`\` newline continues it, a bare newline is part of it) and `[`, `=`s, `[` at the start of a word a `Bracket` token (any number of `=`); `#[=[ ... ]=]` is a bracket comment, any other `#` a line comment.
- Words that touch are the parts of one `Argument`: `-DFOO="bar"` is `Unquoted` `Quoted`, `"a"b` is `Quoted` `Unquoted`, `$<$<CONFIG:Debug>:dbg;"x y">` is three parts. A `(` inside a word is a `ParenGroup` of its own, so `$<$<X:GNU>:(a b)>` is an `Argument`, a `ParenGroup` and an `Argument`; no blank between them is what the layout reads.
- A leading BOM is whitespace; `\r\n`, `\n` and a lone `\r` end a line.

Errors, none of them stops the parse and every token stays in the tree:
- Tokens that are no command (`@VAR@` alone on a line, a stray `)`) are one `ERROR` node up to the next `name(`.
- An unclosed `(` ends its command at the next command that starts at column 0 (name and `(`), else at the end of the file; the closing parentheses are paired in one pass over the tokens before the parse, so a balanced command never stops early. An unterminated quoted text, bracket text or bracket comment ends the same way. A deleted line damages one command: of 600 sampled deletions, 599 lose at most two commands.
- Parentheses nested deeper than 128 levels are one `ERROR` node.

### Block pairing
`core_syntax::block_pair::Pairing` (concept `keyword-block-pairing`) reads the command names of the file in order, case-insensitively, against the table `lang-cmake/src/blocks.rs`: `if`/`elseif`/`else`/`endif`, `foreach`, `while`, `function`, `macro`, `block` with their `end...`. The tree stays flat; `Pairing::word` gives each command its role (opener, arm, closer, plain) and the depth it is printed at (phase 3 and 4 call it again over the commands). A block that cannot be paired is an error on the name of its own command and nothing else changes:
- `unpaired-closer`: no block of its kind is open (`endforeach` for an `if`); the command is plain.
- `unclosed-block`: an opener whose closer never comes (at the opener); a closer of an outer block closes the blocks open inside it.
- `unpaired-arm` (`else` outside an `if`) and `arm-after-last` (`elseif` after `else`).
All 9,066 corpus files pair fully. A deleted `endif` (600 trials) leaves the commands up to the next closer of the enclosing block at the wrong depth: median 2, mean 12.8 commands (135 of 600 none); a deleted `if`: median 3, mean 4.9; one diagnostic on average (`-- --ignored --nocapture pairing`).

Files: `CMakeLists.txt`, `*.cmake` except `*.<ext>.cmake` for the extensions of `exclude-names` in `lang.yaml` (`config.h.cmake`, `foo.pc.cmake`: templates of other languages; `arm.toolchain.cmake` is CMake). The corpus has two more templates that match by name, `run_nvcc.cmake` (`@VAR@` lines) and `clang-format.cmake` (YAML).

## Formatting
Reference tool: gersemi 0.29.2 (MPL-2.0, run as a tool: [../parity/gersemi.yaml](../parity/gersemi.yaml), `scripts/parity.sh format gersemi`). The formatter is `format.rules` plus facts: the block table, the signature table, the `leading_argument_commands` option and the `Name` token (the parser retags the first word of a command so `[case]` reaches it). Every mechanism is shared:

| Piece | Shared construct | Built for it |
|---|---|---|
| Command names in lower case, a known command in the spelling of its table row | `[case]` row on the `Name` token (`#restyle`), spelling from `lang-cmake/src/signatures.generated.rs` | `Parser::bump_as`; the `[case]` row now also reaches `NodeFormatter::value` |
| One command per line at its block depth | `commands` layout (`shapes/commands.rs`, concept `flat-command-list`) over `core_syntax::block_pair` | the layout, `LazyRules::blocks` |
| Arguments on the line when they fit, else one per line, `)` alone; a command whose row says nothing (`bare`) or a call with a comment on a parenthesis | `shapes::delimited_list_around` with a `Fit` list style | nothing |
| Arguments of a known command in keyword sections: a keyword and its values on a line when they fit, else the values one per line one level in; `name value` pairs (`PROPERTIES`); the words of a `COMMAND` filled to the width, a word with a comment on a line of its own; sections nested in a section (`install(TARGETS ... ARCHIVE DESTINATION ...)`); a form chosen by its first words (`list(APPEND NAME` keeps the name, `install(FILES` opens a section); more than four plain values after the positional ones break the call; `set(NAME` keeps its name (option) | `shapes::keyword_call` (concept `keyword-sections`) over `core_syntax::signature::Signature` rows | the layout, `LazyRules::signatures`, `Values::Infix`, `signature::lookup` |
| Conditions of `if`, `elseif`, `while`: one line when it fits, else a line before each `AND` and `OR` (an `Infix` keyword of the row), the operands that follow on that line | the same layout, `operator_sections` | `Values::Infix` |
| Command without a row | same layout: kept as written, lines move with the indent, `)` alone when the call spans lines | the shift model |
| Comments before `else`/`endif`/`endforeach` at the block's body depth | `hold_leading` + `Doc::Outdent` | nothing |
| Blank lines (`max_blank_lines`, none after an opener or before a closer; an arm keeps the blanks around it) | `shapes::gap` | nothing |
| `# gersemi: off` / `# gersemi: on` | grammar `suppress ignore-start/-end` and `core_format::ignore` | nothing |
| Words that touch (`-DX="a b"`, `$<...>`) stay glued | `[space]` rows `* / * : keep` | nothing |

`tabular-rows` and `hanging-argument-list` did not fit: a command's arguments are a flat list, not rows, and gersemi never hangs the first argument for a command it lays out by the generic rule.

### The signature table
`crates/lang-cmake/src/signatures.generated.rs` (343 commands, about 4,400 lines) is generated by `scripts/gen_cmake_signatures.py <CMake source dir>` from the documentation of CMake (pinned in the file header: version 4.5.20261124, commit `4207edd6`; `Help/command/*.rst`, `Help/command/include/FIND_XXX.rst` and the `.. command::` entries of `Modules/*.cmake`, BSD-3-Clause). Only keywords and structure are taken (facts, no prose): the uppercase words of the synopses with what follows each (`Values`: none, one, many, `name value` pairs, a command line, an infix operator), forms (a first keyword that chooses the synopsis: `list(APPEND ...)`; a two-word form only when the first word has several second words: `string(REGEX REPLACE ...)`), sections nested in a section (an alternation group `A|B|C` that holds more keywords; the artifact kinds of `install(TARGETS ...)` from the prose that lists them), the operators of conditions (`<cond1> AND <cond2>`; `elseif` and `while` take those of `if`). An example in a code block counts only when it holds a placeholder (`<x>` or `[x]`). A row is also the list of the known commands: a command is known when it has a row, spelled as the row spells it. gersemi's definitions are not read and not copied.

What CMake's help does not express stays out of the table and is chosen by the layout's generic rules (the causes in [../parity/queue.md](../parity/queue.md)): keywords that only the prose defines (`FILE_SET`, `IN ITEMS`, `ENV` after `HINTS`), which form words keep their first argument on the line (`list(APPEND NAME` does, `file(TO_CMAKE_PATH`, `file(RELATIVE_PATH` and `math(EXPR` do not), and commands that keep more than four plain values on a line.

Equal to gersemi on the corpus: 8,179 of 9,038 files (28 files gersemi rejects; the share is in the parity table); `second-format` 0. Causes of the 859 others, by the first differing hunk: keyword sections 630 (the table lacks what the help does not say), conditions 169 (a nested group that does not fit, `NOT` and `DEFINED` pairs), blank line in front of a comment before a closer 30, unknown commands 30 (user functions: the shift model against gersemi's re-layout). A call with a comment on a parenthesis (`set(# first`, a comment in front of `)`) is laid out by the flat list, which does not keep the name of `set` on the line.

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
| `max_blank_lines` | int 0-10 | `1` | Largest number of blank lines kept between commands and comments. |
| `leading_argument_commands` | list of words | `[set]` | Commands whose first argument stays on the line of the command when the call breaks (`set(NAME` then the values). |
<!-- /generated -->

## Lint
Reference tool: cmake-lint of cmakelang 0.6.13 with its defaults ([../parity/cmake-lint.yaml](../parity/cmake-lint.yaml): 35 codes; C0102, R0914 and W0104 never fire at the defaults). `scripts/parity.sh coverage cmake-lint` runs it on the corpus through `scripts/parity_cmake_lint.py` (one call per file: the tool stops its whole list at a file it cannot parse, 47 of 9,066) and compares `(rule, line)`: 128,501 findings, most of them C0307, C0301 and C0103. The package is GPL-3.0: only its output is compared. The preset `cmake/cmake-lint` turns on the rules Sudsy has for it, at warn (the rules are off by default; the default CMake lint has none): `max-line-length` (C0301, 80), `no-trailing-whitespace` (C0303), `no-hard-tabs` with `indentation_only` (C0306), `statement-indent` (C0307: statements, a closing `)` and comments at depth times 2 over the shared block pairing, `allow_unindented` accepts column 0; cmake-lint does not know `block()`, so files with one differ), `declared-name-pattern` (C0103: the name classes directory, local, cache, internal, parent scope, argument, function and macro, by command position, marker words and the enclosing `function` or `macro`; a `${...}` reference is cut out before the match, a macro body counts as parent scope), `statement-blank-lines` (C0305) and `documented-command` (C0111, C0112: a comment of text right above `function` and `macro`; only line comments document). Coverage on the corpus (same line, 47 unreadable files out of both sides): the share is in the Language support table; open: C0113 and E11xx (command signatures), R09xx, W0105, W0106, C0327, C0304, C0114, C0201, C0321, W0101 (`planned` in the catalog).
