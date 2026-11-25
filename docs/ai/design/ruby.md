# Design: Ruby format after RuboCop's Layout cops

Goal: `sudsy format` gives at least 85% of the corpus files the bytes RuboCop gives them (`rubocop --only Layout -a`, default configuration). The parser, the tree, the verbatim formatter and the lint are in [../formatting/ruby.md](../formatting/ruby.md); this note holds what is open and is deleted when the format leg is ready.

## Measured
RuboCop 1.92.3, `TargetRubyVersion: 3.4`, `NewCops: disable`, 18,958 files (Rails, Discourse, RuboCop, Jekyll; `tests/conformance/fetch-ruby-corpus.sh`).
- RuboCop leaves 13,321 files (70.27%) as they are, so verbatim output stands at 70.27%; 85% is 16,115 files, 2,794 of the 5,637 that change.
- RuboCop is a linter with layout fixes: it keeps the source's line breaks, joins nothing and re-wraps nothing (exceptions: `BlockEndNewline`, `MultilineBlockLayout`, the brace layout cops, `DotPosition`, `LineLength`). A formatter that re-wraps differs in every file it touches.
- A second pass of RuboCop changes 0 files. 7 files change meaning (moved lines inside a multi-line string, regex or heredoc; blanks deleted in a `<<-` heredoc) and 1 ends in "Infinite loop detected": these 8 are `tool-wrong` candidates, each entry named and proven.

Files completed when every cop that corrects a file belongs to a chosen group (upper bounds; running RuboCop with only a prefix's cops agreed within 3 files, on a grouping that held the two alignment groups as one):

| Group added | Equal | Share |
|---|---|---|
| verbatim | 13,321 | 70.27% |
| blank lines | 14,479 | 76.37% |
| chain and operator indent | 15,281 | 80.60% |
| block indent | 15,976 | 84.27% |
| argument and literal alignment | 16,558 | 87.34% |
| token spacing | 17,153 | 90.48% |
| heredoc indent | 17,770 | 93.73% |
| comment text, trailing whitespace | 18,001 | 94.95% |
| line length | 18,651 | 98.38% |
| brace layout | 18,958 | 100% |

The bound says which cops matter, not that our version reproduces them. The larger risk is the 13,321 files that are right today: each 1% of them changed costs 0.7 points.

## Design
A line-preserving formatter in `core-format`, not a printer: every source line stays, and the formatter sets its indentation, the blank lines between lines and the blanks between tokens. HCL (`crates/lang-hcl/src/format.rs`, 250 lines) and PowerShell (496 lines) are such formatters today, each with its own loop. Ruby would be the third copy, so the loop is extracted first and Ruby is written on it as data. `lang-ruby` gets no hand-written formatting Rust.

| Need | Built on | Second language |
|---|---|---|
| line loop: lines of tokens, protected lines (multi-line literals, heredoc bodies, `__END__`, `=begin`) | extracted from HCL and PowerShell | HCL, PowerShell (moved onto it) |
| indentation by nodes that end in a keyword (`def`..`end`, `do`..`end`, `if`..`else`..`end`; modifier forms open nothing), chains and operands | `stream-block-indent` (`core_format::stream_indent`) | none yet: kind sets as a plain option until one needs it |
| continuation lines: under the first `.`, under the operand, one step from the statement; arguments, hash pairs and arrays aligned with the first element or one step from the line | `indent-frames`, `column-alignment` | check clang-format `AlignOperands`, ktlint chains; if none uses it: shared Rust with an option |
| blank lines: after a guard clause, after leading magic comments, around an access modifier, none at the edges of a body | `stream-blank-lines` (`core_format::stream_blank`) | none: PHP's `blank_line_before_statement` (a blank before a jump) and `blank_line_after_opening_tag` are rules of its tree printer, not of the token stream, and share no fact with these |
| token spacing, extra blanks kept only where they align with an adjacent line | `token-spacing-rows` | already shared |
| `<<-` and `<<` to `<<~` with the body re-indented, closing label | `raw-string-reindent` | PHP (`heredoc_indentation`) |
| `#x` to `# x` with the `#!`, `#:` exceptions | `token-rewrite-rows` | already shared |

Every need is checked against the kit index before it is built. Each choice of a cop's `EnforcedStyle` that we do not follow by default is a named option only if a preset needs it.

Where blanks carry meaning, the node decides and a token pair does not: `foo (1)`, `puts -x`, `a [1]`, `foo /x/`, `{` of a block against `{` of a hash, `%w[ a ]`, a `\` at a line end (no blank after it). A line inside a multi-line literal is never moved, even where RuboCop moves it.

## Phases
Each phase ends with a landed commit and the run on the whole corpus: share equal, files right before and wrong after (the regression list, a ratchet that may only go down), second-format 0, files whose Prism tree changes 0.
1. Runner and ratchets (RuboCop run once over the copied tree, project configurations ignored, output cached in the slot by version and corpus revision), the line loop extracted with HCL and PowerShell on it (their output unchanged), Ruby on it with blank-line rows and token spacing rows. The bound is measured by the runner first (blank lines alone: 76.4%). **Checkpoint**: at most 0.5% of the 13,321 files regress; otherwise stop.
2. Block indentation, then chain and operator indentation.
3. Argument and literal alignment. **Checkpoint**: the share against the bound of 90.5% with token spacing; if a phase lost more than 40% of its bound, rank the causes again before going on.
4. Heredocs and comment text if 85% is not reached or the margin is under one point; `differences.yaml` with each `tool-wrong` proven; support row, options, speed row and numbers.
Line length and brace layout break lines and are not part of this plan.

Budget: Ruby's own format metrics are free until the first real formatter has landed; HCL and PowerShell format up to +5% (moved code); everything else +1%.
