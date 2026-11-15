# SQL (`lang-sql`)

## Parser
[sqruff](https://github.com/quarylabs/sqruff)'s `sqruff-lib-core`, `sqruff-lib-dialects` and (the fixer) `sqruff-lib` 0.44.0 (Apache-2.0), pinned to the exact version in the root `Cargo.toml` (dependabot ignores them; a bump is a deliberate change of `grammars/sql.grammar` and `crates/lang-sql/src/adapter.rs`, followed by the conformance run). One language `sql` covers `*.sql`; the dialect is the option `dialect`, one of the four offered: `ansi` (default), `postgres`, `mysql`, `tsql` (the fixer `sqruff-lib` depends on `sqruff-lib-dialects` with its default features, so every dialect of sqruff is compiled in; the other ones are not offered because they hang or panic). The dialect travels in `SourceFile::language_version`; a dialect is built on a thread's first file of it.

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
| `comma_position` | `trailing`, `leading` | `trailing` | Where the comma of a list that breaks goes: at the end of a line or at the start of the next. |
| `keyword_case` | `consistent`, `upper`, `lower`, `capitalise`, `preserve` | `consistent` | Capitalisation of keywords: one spelling throughout the file (the first one's), or the named case. |
| `identifier_case` | `consistent`, `upper`, `lower`, `capitalise`, `preserve` | `consistent` | Capitalisation of unquoted identifiers. |
| `function_case` | `consistent`, `upper`, `lower`, `capitalise`, `preserve` | `consistent` | Capitalisation of function names. |
| `literal_case` | `consistent`, `upper`, `lower`, `capitalise`, `preserve` | `consistent` | Capitalisation of `NULL`, `TRUE` and `FALSE`. |
| `type_case` | `consistent`, `upper`, `lower`, `capitalise`, `preserve` | `consistent` | Capitalisation of data types. |
<!-- /generated -->

## Formatting
The format is `sqlfluff fix` with the layout rules (LT01-LT15: spacing, indentation, line breaks, commas, line length, trailing newline) and the capitalisation rules (CP01-CP05), run by sqruff's own fixer (`sqruff-lib` 0.44.0, pinned with the parser; `src/fix.rs`) with sqlfluff's defaults. No layout code of ours: the options become sqruff's configuration (`line_width` is `max_line_length`, `indent_*` the `indentation` section, `comma_position` the `comma` layout, the `*_case` options the policy of CP01-CP05, `preserve` leaves that rule out).

The fixer works on its own parse, so the formatter is a source rewrite (`NodeFormatter::rewrite_source`) and the layout step prints the rewritten file as written:
- the fix is accepted only when the safety check holds against the input: the rewritten text parses without errors and has the input's tokens and nodes, comments and string literals unchanged (`core_format::safety::verify`; keywords and unquoted identifiers compare in lower case, because the capitalisation rules change them). Otherwise (sqruff's fixer has no such guard; for example it moves text into a nested Postgres block comment) the file stays as written;
- a panic in the fixer is caught and leaves the file as written;
- `end_of_line: auto` keeps the file's first line break (sqruff works on LF);
- keywords, identifiers, functions and types are cased by `consistent`: the first spelling in the file wins, as in sqlfluff.

Not offered: the other `indentation` and `layout` settings of sqlfluff (`indented_joins`, `layout.type.*`, ...), `ignore_words`, the `pascal`/`camel`/`snake` identifier policies and templating; they are deferred in [the catalog](../parity/sqlfluff.yaml).

## Parity
`scripts/parity.sh format sqlfluff` (sqlfluff 4.5.0 `fix --rules layout,capitalisation` against `sudsy format`, one dialect per fixture): sqlfluff's dialect fixtures and PostgreSQL's regression inputs of the conformance corpus. Identical output on 635 of the 867 files sqlfluff accepts (73%; 1,121 files, 254 of them refused by sqlfluff itself: psql commands, template tags). Every difference is one of four named gaps in `tests/parity/sqlfluff/differences.yaml`, all of them sqruff's against sqlfluff, so Sudsy cannot close them without its own layout code: files sqruff's dialect does not parse (`parse-gap`), fixes dropped by the guard (`fix-dropped`), a layout sqruff computes differently (`layout-gap`: compound statement bodies and `DELIMITER` in MySQL and T-SQL, `REFERENCES t (c)`, statements that stay on one line, block-comment continuation lines) and capitalisation (`case-gap`).

## Conformance
`bash tests/conformance/fetch-sql-corpus.sh`, then `cargo test --profile corpus -p lang-sql --test integration -- --ignored --nocapture conformance::`: sqlfluff's dialect fixtures (ansi, postgres, mysql, tsql) and PostgreSQL's regression inputs; every file prints back byte for byte and satisfies the trivia rule, and the files with parse errors and panics are counted per dialect. `conformance::format_is_stable` formats every file that parses twice (stable output, safety check passed).
