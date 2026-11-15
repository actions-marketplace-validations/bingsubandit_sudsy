# SQL (`lang-sql`)

## Parser
[sqruff](https://github.com/quarylabs/sqruff)'s `sqruff-lib-core` and `sqruff-lib-dialects` 0.44.0 (Apache-2.0), pinned to the exact version in the root `Cargo.toml` (dependabot ignores them; a bump is a deliberate change of `grammars/sql.grammar` and `crates/lang-sql/src/adapter.rs`, followed by the conformance run). One language `sql` covers `*.sql`; the dialect is the option `dialect`, one of the four compiled in: `ansi` (default), `postgres`, `mysql`, `tsql` (the other dialects of sqruff are not built: their features are off in the root `Cargo.toml`). The dialect travels in `SourceFile::language_version`; a dialect is built on a thread's first file of it.

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
<!-- /generated -->

## Formatting
The formatter prints the file as written.

## Conformance
`bash tests/conformance/fetch-sql-corpus.sh`, then `cargo test --profile corpus -p lang-sql --test integration -- --ignored --nocapture conformance::`: sqlfluff's dialect fixtures (ansi, postgres, mysql, tsql) and PostgreSQL's regression inputs; every file prints back byte for byte and satisfies the trivia rule, and the files with parse errors and panics are counted per dialect.
