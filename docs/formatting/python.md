# Python (`lang-python`)

## Parser
[`ruff_python_parser`](https://github.com/astral-sh/ruff) 0.0.14 (MIT) with `ruff_python_ast` and `ruff_text_size`, pinned to the exact version in the root `Cargo.toml` (the version Pyrefly pins; dependabot ignores the `ruff_*` crates). The crates are 0.0.x and change weekly, so a bump is a deliberate change of `grammars/python.grammar` and `crates/lang-python/src/adapter.rs`.

`adapter.rs` builds our tree (the shape of `lang-js`'s) from three inputs:
- the lexer's tokens, one token kind per ruff `TokenKind` (names as ruff's: `Name`, `Lpar`, `If`, `String`, `FStringMiddle`, ...). Comments are `Comment` trivia; logical and non-logical newlines, indents and dedents are `Newline` and `Whitespace` trivia, and a backslash that ends a line is `Continuation`;
- the AST's node spans, from one `SourceOrderVisitor` walk: each AST node is a node of the tree, named as in ruff (`StmtFunctionDef`, `ExprCall`, `Arguments`, `Parameters`, ...), `ModModule` is the root. A node with no token (a name error recovery adds) is an empty node;
- the text between two tokens, split into trivia tokens and attached by the shared trivia rule. A byte-order mark is leading trivia; text that is none of these is an `Unknown` token and an error.

ruff's expression ranges exclude parentheses, so the adapter makes a `Parenthesized` node (tagged `#transparent`) around each parenthesized expression or pattern, one per pair, nested for `((x))`. The parentheses of a call or of a class's bases belong to `Arguments` and make none.

Errors: ruff recovers from syntax errors and its errors cascade, so a file reports the first error of each statement (code `syntax-error`), and the version errors of the option `target_version` (`lazy import` before 3.15) are syntax errors. A file with errors is reported, not formatted.

## Formatting
The output is `ruff format` 0.17.4's (Black's style) under its defaults; `scripts/parity.sh format ruff` compares the two on the pinned corpus (black's test cases, ruff's formatter fixtures, six Black-formatted projects; numbers in [../performance/numbers.md](../performance/numbers.md)) and `tests/parity/ruff/differences.yaml` ratchets each group. Code: `crates/lang-python/src/format/` (`stmt` blocks, blank lines and comments, `paren` where parentheses stand, `expr`/`list`/`op` the expressions, `literal` and `docstring` strings and numbers).
- Statements: blank lines as ruff (two around top-level `def` and `class`, one nested, one after imports and after a module or class docstring, up to two or one kept elsewhere, none at the start of a block except in a function body and before a nested `def`); a `...` body stays on the header line; `a; b` is two lines; `class A():` loses its parentheses.
- Brackets: calls, collections, parameters, subscripts and `with` items split at their brackets, the items on one line when they fit there and one per line (a comma after the last) when they do not; a trailing comma in the source keeps the list expanded (`magic_trailing_comma`); imports in parentheses are always one per line.
- Parentheses: redundant ones go (`x = (1)`, `for (a, b) in c`, `return (a + b)`) and come back when the expression does not fit (`x = (\n    a + b\n)`); an expression that ends in a bracket splits there instead (`x = f(a) + g(\n    b\n)`), a call that fits in parentheses after `=` is parenthesized when the line up to its first bracket does not fit. Calls chained with two or more `.attr` after a call or subscript break before each of them. Operators break before the operator; `a**b` of simple operands has no spaces; slices get spaces around `:` when an operand is not simple.
- Strings: quotes follow `quote_style` (the other quote when that needs fewer escapes; raw and triple-quoted strings keep theirs unless safe), prefixes are lowercase except `R`, `u` goes; numbers are lowercase with upper-case hex digits; implicit concatenations of plain strings are joined when the line holds them, else one part per line. Docstrings: the first statement of a module, class or function body; quotes `"""`, indentation of the lines after the first re-indented to the block, trailing blanks removed.
- Comments: `#comment` becomes `# comment` (not `#!`, `#:`, `##`), two spaces before a trailing comment (its text counts for the width unless it is a tool pragma: `noqa`, `type:`, `pylint:`, ...), comments at the end of a block stay in it. `# fmt: off` / `# fmt: on` and a trailing `# fmt: skip` (simple statements) and `sudsy-ignore`, `sudsy-ignore-start`, `sudsy-ignore-end` keep statements as written.
- The text is prepared before formatting (comments, `class A():`, see `PythonFormatter::rewrite_source`); the safety check follows it: a pair of parentheses is not compared (`Lpar`, `Rpar` are optional and `Parenthesized` is transparent), a string expression is compared by prefix and body (so joining implicit concatenations and changing quotes pass), a docstring without its indentation.
- Not done: code in docstrings (`docstring-code-format`), quotes inside f-string replacement fields, comments in unusual places (between `lambda` and its body, after a bracket before a first item on its own line), `# fmt: skip` on compound headers and `# fmt: off` inside brackets, preview style.

<!-- generated:options -->
| Option | Values | Default | Description |
|---|---|---|---|
| `line_width` | int 20-1000 | `88` | Preferred maximum line width in columns. |
| `indent_size` | int 1-16 | `4` | Columns per indentation level (also the width of a tab). |
| `indent_style` | `space`, `tab` | `space` | Indent with spaces or tabs. |
| `final_newline` | bool | `true` | End the file with a newline. |
| `end_of_line` | `auto`, `lf`, `crlf` | `auto` | Line break style of the output: `lf`, `crlf`, or `auto` to keep the file's first line break. |
| `quote_style` | `double`, `single`, `preserve` | `double` | Quotes of strings (and of f-string and t-string literals): `double` or `single` rewrite a string to that quote unless the other one needs fewer escapes, `preserve` keeps what the source has. |
| `magic_trailing_comma` | `respect`, `ignore` | `respect` | `respect` keeps a collection, call or parameter list that has a trailing comma expanded one item per line; `ignore` joins it when it fits and removes the comma. |
| `target_version` | `3.9`, `3.10`, `3.11`, `3.12`, `3.13`, `3.14`, `3.15` | `3.14` | The Python version whose syntax the file must follow: newer syntax is a syntax error (`lazy import` needs 3.15). |
<!-- /generated -->
