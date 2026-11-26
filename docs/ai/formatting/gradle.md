# Gradle (`lang-gradle`)

## Parser
tree-sitter through `core-treesitter` (mapping in `crates/lang-gradle/lang.yaml`, grammar `grammars/gradle.grammar`) over the fork `grammars/tree-sitter-groovy` of `dekobon-tree-sitter-groovy` 0.6.0 (Apache-2.0 OR MIT; changes and numbers in its `FORK.md`). Chosen on the 1,538-file capped corpus of [../design/gradle.md](../design/gradle.md) against the murtaza64, loganprice and amaanq grammars and a hand-written lexer: 89.3% of files error-free (the others 80.6%, 71.9%, 46.2%), 4 of 70 probes failing, the only one with an active upstream and a scanner for GStrings, slashy and dollar-slashy strings. The fork parses command expressions with several or named arguments after a name or dotted name (`dependsOn a, b`, `inputs.property "k", v`, `attributes 'k': v`, `def t = project.tasks.create "x", Jar`) and Java-style lambdas: 98.6% of the capped corpus and 99.4% of all 6,481 files (the 39 others are `<% %>` or `@x@` templates or rare forms). Speed 12.7 MB/s. A string is one token however many `${}` it holds; a file with an error node is not formatted. `*.gradle` only; `*.gradle.kts` is Kotlin.

## Formatting
Rows only (`format.rules`), written line by line: every line keeps its break and is indented by the brackets open in front of it (`frame-indent`), 4 spaces by default (`indent_size`, `indent_style`; a tab in the source is converted). Comments and strings are printed as written.
- No blank line after an opening bracket that ends a line or before a closing bracket that starts one (`trim-bracket-edges`); blank lines collapse to `max_blank_lines` (1); one line break ends the file.
- One blank after a comma or `;` and none before; none inside `(` `)` `[` `]`, one inside the braces of a one-line closure, one before `{`; one around `=` and the compound assignments, ternary `?` and `:`; one after the `:` of a named argument or map entry and none before; one after `if`, `for`, `while`, `switch`, `catch`, `synchronized`.
- Left as written: quotes, parentheses of a dependency declaration, blanks around binary operators, a trailing `;`, comment text, line width.
- The rows are the decision table of [../design/gradle.md](../design/gradle.md): pooled held-out agreement 87.1%; the indent width (4 against 2 in 41% of occurrences) is the one row under 80% and the reason for the option. No reference formatter runs; Spotless greclipse is the comparison.

## Conformance
The ignored test `conformance` of `crates/lang-gradle/tests/gradle.rs` runs over the 6,481 files of `tests/conformance/fetch-gradle-corpus.sh`: round trip, meaning and idempotency hold on every file that parses; 39 files do not parse (stable 99.4%, ratchet in `tests/parity/stability/differences.yaml`). Agreement 87.1% (`tests/parity/gradle/differences.yaml`).

## Options
<!-- generated:options -->
| Option | Values | Default | Description |
|---|---|---|---|
| `indent_size` | int 1-16 | `4` | Columns per indentation level (also the width of a tab). |
| `indent_style` | `space`, `tab` | `space` | Indent with spaces or tabs. |
| `final_newline` | bool | `true` | End the file with a newline. |
| `end_of_line` | `auto`, `lf`, `crlf` | `auto` | Line break style of the output: `lf`, `crlf`, or `auto` to keep the file's first line break. |
| `max_blank_lines` | int 0-10 | `1` | Largest number of blank lines kept between lines. |
<!-- /generated -->
