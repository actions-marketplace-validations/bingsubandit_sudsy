# SQL (`lang-sql`)

## Parser
[sqruff](https://github.com/quarylabs/sqruff)'s `sqruff-lib-core` and `sqruff-lib-dialects` 0.44.0 (Apache-2.0), pinned to the exact version in the root `Cargo.toml` (dependabot ignores them; a bump is a deliberate change of `grammars/sql.grammar` and `crates/lang-sql/src/adapter.rs`, followed by the conformance run). One language `sql` covers `*.sql`; the dialect is the option `dialect`, one of the four offered: `ansi` (default), `postgres`, `mysql`, `tsql` (the fixer `sqruff-lib` depends on `sqruff-lib-dialects` with its default features, so every dialect of sqruff is compiled in; the other ones are not offered because they hang or panic). The dialect travels in `SourceFile::language_version`; a dialect is built on a thread's first file of it.

sqruff lexes and parses a whole file into segments (raw leaves and nodes named by sqlfluff's segment types). `adapter.rs` builds our tree from them:
- the leaves in order are the tokens (`Whitespace`, `Newline`, `LineComment`, `BlockComment`; `Keyword`, `Identifier`, `QuotedIdentifier`, `String`, `Number`, `Operator`, and the punctuation `( ) , ; .`); zero-width leaves (`Indent`, `Dedent`, `EndOfFile`) are dropped, a leaf that does not equal the text at its place fails the file;
- a segment type is a node when `grammars/sql.grammar` has a node of its name in Pascal case (`select_statement` is `SelectStatement`); any other `*_statement` is a `Statement`; the other types are transparent (their leaves stay, the node does not). `Unparsable` is an `ERROR` node and a parse error at its span;
- trivia are attached by the shared rule, whatever sqruff's tree says (`core_syntax::trivia::attach`), and a zero-width `Eof` token ends the root.

Failure modes, all leaving the file unchanged (it is one `Unknown` token and one parse error, which the formatter and the linter refuse):
- sqruff panics on some inputs (an `unimplemented!` or `unwrap` in a dialect): the parse runs under `catch_unwind` and the error says the parser failed on the file;
- a file over `MAX_BYTES` (512 KB) is not parsed: sqruff parses 0.3 to 1 MB/s, so the limit keeps one file to about 1.6 s (peak memory is about 650 bytes per input byte);
- a nesting deeper than sqruff's `DEFAULT_MAX_PARSE_DEPTH` (600) is sqruff's own error.
Dialects whose parser hangs (`soql`) or panics often (`redshift`, `sqlite`, `starrocks`, `trino`) are not offered.

## Options
<!-- generated:options -->
| Option | Values | Default | Description |
|---|---|---|---|
| `dialect` | `ansi`, `postgres`, `mysql`, `tsql` | `ansi` | The SQL dialect the file is parsed as. |
| `line_width` | int 20-1000 | `80` | Preferred maximum line width in columns. |
| `indent_style` | `space`, `tab` | `space` | Indent with spaces or tabs. |
| `indent_size` | int 1-16 | `4` | Columns per indentation level (also the width of a tab). |
| `final_newline` | bool | `true` | End the file with a newline. |
| `end_of_line` | `auto`, `lf`, `crlf` | `auto` | Line break style of the output: `lf`, `crlf`, or `auto` to keep the file's first line break. |
| `comma_position` | `trailing`, `leading` | `trailing` | Where a comma goes when a list is broken over lines: at the end of a line, or at the start of the next one. |
| `keyword_case` | `consistent`, `upper`, `lower`, `capitalise` | `consistent` | Case of keywords: `consistent` follows the first one written (all that follow are brought to its case once one differs), the others set it. |
| `identifier_case` | `consistent`, `upper`, `lower`, `capitalise` | `consistent` | Case of unquoted identifiers: `consistent` follows the first one written (all that follow are brought to its case once one differs), the others set it. |
| `function_case` | `consistent`, `upper`, `lower`, `capitalise` | `consistent` | Case of function names: `consistent` follows the first one written (all that follow are brought to its case once one differs), the others set it. |
| `literal_case` | `consistent`, `upper`, `lower`, `capitalise` | `consistent` | Case of `NULL`, `TRUE` and `FALSE`: `consistent` follows the first one written (all that follow are brought to its case once one differs), the others set it. |
| `type_case` | `consistent`, `upper`, `lower`, `capitalise` | `consistent` | Case of data types: `consistent` follows the first one written (all that follow are brought to its case once one differs), the others set it. |
<!-- /generated -->

## Formatting
sqruff is the parser only: layout is Sudsy's own code (`src/layout/`) on the parse tree, following sqlfluff's layout rules (LT01 to LT15) with its defaults ([the catalog](../parity/sqlfluff.yaml)). The tree is flattened to blocks (tokens) and the points between them (spacing, line breaks, indentation); `space.rs` sets the spacing (LT01, LT06, LT07, LT10, LT11 and so on), `breaks.rs` the line breaks (LT08, LT09, LT12, LT13, LT14, LT15), `indent.rs` the indentation (LT02, LT03). The rules run in turn until the layout stops changing, as `sqlfluff fix` does, and the result is printed line by line. `wrap.rs` breaks lines over `line_width` (LT05: the outermost indent points, or the operators and commas by precedence, as sqlfluff picks them), `case.rs` sets the case of words (CP01 to CP05), `blocks.rs` adds the indents that sqruff's MySQL (procedural blocks) and T-SQL (`IF`, `ELSE`, `WHILE` bodies) dialects leave out. `comma_position` (LT04) puts the commas of a broken list at the end of the line (default) or the start of the next one. Not yet written: comment placement; those parts stay as written.

### Case
Five classes of words have a case option: keywords and binary operators (`keyword_case`), unquoted identifiers (`identifier_case`), function names including `CURRENT_DATE` and the like (`function_case`), `NULL`/`TRUE`/`FALSE` (`literal_case`) and data types (`type_case`). `upper`, `lower` and `capitalise` set it. `consistent` (the default, sqlfluff's) reads the words of a class in order and keeps the cases the words so far agree on (upper, lower, capitalised, in that preference); the word that rules out the last one, and every one after it, is set to the last case that was still possible, so the whole file ends in one case. The parser's word types decide the class (`adapter.rs`): a keyword inside a data type is a type, the last part of a qualified function name (a user's function) and datatype methods are no class and stay as written. The safety check compares these words lowercase.

## Lint
Rules of sqlfluff's aliasing (AL), ambiguous (AM), convention (CV) and structure (ST) families that catch bugs or dead code are on by default (`null-comparison`, `distinct-group-by`, `unused-cte`, `duplicate-table-alias`, `self-alias`, `union-distinct`, `else-null`); the style ones (`implicit-alias`, `count-star`, `prefer-coalesce`, `right-join`) are off. Most are pattern rules over the node kinds in `crates/lang-sql/rules/`; the ones that look at names are Rust (`src/rules.rs`). Layout (LT) and capitalisation (CP) are the formatter's, so they have no rules. Names are compared lowercase without quotes; no rule resolves columns or schemas, so sqlfluff's reference (RF) rules are planned ([the catalog](../parity/sqlfluff.yaml)).

## Parity
`scripts/parity.sh format sqlfluff` (sqlfluff 4.5.0 `fix --rules layout,capitalisation` against `sudsy format`, one dialect per fixture): sqlfluff's dialect fixtures and PostgreSQL's regression inputs of the conformance corpus. Identical to sqlfluff's on 674 of the 867 files sqlfluff accepts (ansi 202, postgres 121, mysql 206, tsql 108, PostgreSQL regress 37; 1,121 files, 254 of them refused by sqlfluff itself: psql commands, template tags); a second format changes none. Every difference is a named gap in `tests/parity/sqlfluff/differences.yaml`: files our parser does not accept (`parse-gap`), and the layout not written yet (`layout-gap`).

## Conformance
`bash tests/conformance/fetch-sql-corpus.sh`, then `cargo test --profile corpus -p lang-sql --test integration -- --ignored --nocapture conformance::`: sqlfluff's dialect fixtures (ansi, postgres, mysql, tsql) and PostgreSQL's regression inputs; every file prints back byte for byte and satisfies the trivia rule, and the files with parse errors and panics are counted per dialect. `conformance::format_is_stable` formats every file that parses twice (stable output, safety check passed).
