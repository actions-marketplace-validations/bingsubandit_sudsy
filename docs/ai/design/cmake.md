# CMake: implementation plan

Decided by the user: a hand-written parser that reuses the shared parse code as far as it goes. Both legs: format against gersemi, lint against cmake-lint's defaults. Measured corpus: 9,066 files of LLVM, CMake, Qt, VTK, OpenCV, extra-cmake-modules and protobuf (22.8 MB). Done when both legs are at 85% on it; the format reach measured by a model is about 86% with the keyword-section layout, 61% without, so phase 5 decides the leg.

Each phase ends with one finish run and a landing; a checkpoint (C) stops for the lead.

## Facts that shape the plan
- A file is a flat list of commands `name(args)`. Blocks are pairs of commands (`if`/`elseif`/`else`/`endif`, `foreach`/`endforeach`, `while`, `function`, `macro`, `block`), each with its own closer. An argument is bracket, quoted (with `\` newline continuation) or unquoted, with `${}` references and nested generator expressions inside, and parenthesized groups. Adjacent arguments without a blank are one token run and stay glued (87 files, 180 places).
- Command names are case-insensitive; there are no keywords in the lexer. A word's role comes from the command's signature.
- `*.<ext>.cmake` (`config.h.cmake`, `foo.pc.cmake`) are templates of other languages: the file matcher leaves them out by name; they are not handled by a parser special case.
- No licence trap: the signature table is written from CMake's own documentation (BSD); gersemi's table (MPL-2.0) and cmake-lint's (GPL-3.0) are never copied, only their output is compared.

## Reuse map (from the code, not the index)
| Piece | Shared code | Verdict |
|---|---|---|
| tokens, trivia, cursor, depth guard, one diagnostic per site | `core-syntax/parse.rs` (`Lexer`, `Parser`; as lang-json uses it) | as is |
| size, nesting and panic guards | `provider-api/guard.rs` | as is |
| strings | `string-literal-model` | as is |
| keyword blocks, parse | `core-syntax/block_parse.rs` | does not fit (one closer word `END`, words instead of commands, `THEN`/`DO` openers, no nodes); not extended. `block_pair.rs` pairs the command names (phase 2) |
| indent of keyword blocks | `core-format/block_frames.rs` | fits only after phase 2; its nesting is computed from words |
| conditions | `core-syntax/parse.rs` Pratt (`climb`) | only for the layout of `if` and `while`; the lint needs none |

## Phases
1. **Parser.** `grammars/cmake.grammar`, lexer and parser on the kit, about 450 lines: file, command, argument list, the three argument kinds, variable and generator expressions as nested nodes only where a rule reads them, paren groups, comments (line, bracket). Recovery: an unclosed `(` or quote ends at the next line that starts a command at column 0 or at a known indent; a deleted line damages one command. Compare with tree-sitter-cmake on the corpus: every file round-trips; parse failures at most the 31 templates; speed at least 30 MB/s (tree-sitter 3.7, cmake-fmt 64); the one-line-deletion test at least as good as cmake-fmt (588 of 600 lose at most 2 commands). If lexer code that six languages each have is needed (the LOCAL row `source-lexer`), the piece CMake needs moves to core-syntax in the same step and at least one of them adopts it. **C1:** corpus numbers, line counts, what moved to shared code.
2. **Block pairing prototype.** Built (a) and (b), measured, kept (a), deleted (b). (a): the tree stays the flat command list; `core_syntax::block_pair::Pairing` is fed the command names in order, from a table of `Block`s (opener, arms, last arm, closer; CMake's is `lang-cmake/src/blocks.rs`), gives each command its role and depth and reports an unpaired block as a diagnostic on its own command (`unclosed-block`, `unpaired-closer`, `unpaired-arm`, `arm-after-last`). (b) was `block_parse` with a closer per frame, block nodes in the tree and cursor hooks. Equal damage on the 600-trial deletion test; (a) wins on lines of the language crate (28 against 120: the `Cursor` trait is shaped for SQL, so CMake stubbed eight of its methods), keeps the grammar and every walker flat, and has no recursion. The layout twin (`keyword-block-frames`) follows the same table in phase 4.
3. **Lint first** (built, 97.2% on the corpus): the shared rules `statement-indent` (C0307, the block pairing's depth, `Step::column`, a fix), `declared-name-pattern` (C0103: classes from command position, marker words and enclosing blocks, written as option rows of the preset), `statement-blank-lines` (C0305), `documented-command` (C0111, C0112) and options of `max-line-length`, `no-trailing-whitespace` and `no-hard-tabs` (`indentation_only`, C0306); the language crate holds only the block table behind `RuleHooks::blocks`. Left: C0113 and E11xx from the signature table (`missing_arguments` (PowerShell) gets its second user: extend its table type instead of writing a new one), R0915/R0912/R0911/R0913 (`max-family`), W0105, W0106. Unverified in the measurement: E0103, E0108, E0109, E0011, E0012, C0102, C0202, R0914, W0104.
4. **Format, generic tier** (61%): command names by `token-case`, one line when it fits in the width, else one argument per line with `)` on its own line (`hanging-argument-list`, with the option for the lone parenthesis), unknown commands keep their rows (`tabular-rows`), comments, indent from phase 2, `# gersemi: off`.
5. **Keyword sections** (to about 86%): layout of a command by its signature (positionals, options, one-value and multi-value keywords, nested sections, forms chosen by the first keyword). The table of facts is plain data read by a shared module next to `missing_arguments`; the width decision is shared Rust. The table is `core_syntax::signature::Signature` (the row of PowerShell's `missing-arguments` rule is the same type), generated from CMake's own help; the layout is `keyword-sections` in `core-format`.
6. **Conditions and comments in argument lists** (to the rest): `binary-chain-layout` for `if`/`while`; pairs through `key-value-layout`; `command_line` split.
7. **Generalization step**, performance rows, docs, VS Code.

## Rules for every phase
Known parser behaviour differences against CMake itself are fixtures (bracket argument levels, `\` newline, `;` in arguments, legacy `-DFOO="bar"` splits, non-ASCII). Every phase reports the corpus count that round-trips and the second-format count (0). New-language budgets apply.
