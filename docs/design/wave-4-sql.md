# Design: SQL

Decided plan with one open measurement (step 0), built in the steps of section 5; each step moves what it builds into `docs/formatting/sql.md`, and this note is deleted when the last step lands.

Goal: the formatter and linter sqlfluff and sqruff users switch to: sqlfluff's layout and capitalisation rules (which together are its "formatter"), its bug-catching rules, per-dialect parsing, and migration from `.sqlfluff` / `pyproject.toml [tool.sqlfluff]` / `.sqruff`.

## Summary
1. Parser: **sqruff's `sqruff-lib-core` + `sqruff-lib-dialects`** (Apache-2.0, crates.io, 27 of sqlfluff's 28 dialects), pinned exactly, behind a segment-tree-to-lossless-tree adapter. Fallback if step 0 rejects it: tree-sitter-sql (`tree-sitter-sequel`, MIT) as a vendored grammar for an `ansi`-like dialect only.
2. One language `sql` for `.sql`; the dialect is an option (`dialect`), not a language: sqlfluff requires one, Sudsy defaults to `ansi`.
3. Formatter: sqlfluff's layout rules (LT01-LT15) and capitalisation (CP01-CP05) as the format, with sqlfluff's defaults (`max_line_length` 80, 4 spaces, trailing commas, capitalisation `consistent`).
4. Lint: sqlfluff's non-layout rules (AL, AM, CV, RF, ST, ...) by usage; zero-config set = the bug-catching ones (ambiguous, references, structure), style rules off.
5. Templated SQL (Jinja, dbt) is not formatted or linted in the first steps: a file containing `{{`, `{%` or `{#` is reported as templated and skipped (named difference), until a later step.

## 0. Spike (an agent, before step 1; measured, not committed)
Measure on sqlfluff's `test/fixtures/dialects` (each `.sql` with its `.yml` parse tree, MIT) and the PostgreSQL regression `sql/` files:
- sqruff-lib-core: round-trip (concatenated raw segments == input) per dialect; parse failures (`Unparsable`) vs sqlfluff's `.yml`; parse speed MB/s; peak memory; binary added to `sudsy.exe`; crates added and licenses (ICU, fancy-regex).
- tree-sitter-sequel: the same numbers on the ansi/postgres/mysql/sqlite fixtures.
Decide: sqruff unless round-trip fails, it is under 5 MB/s, or it adds over 4 MB.

**Measured** (Linux arm64 Graviton, 8 cores, rustc 1.99.0, release with LTO and `codegen-units = 1`, parse = lex + parse of the whole file from memory, warm; sqruff-lib-core/-dialects 0.44.0 with all 27 dialects, tree-sitter-sequel 0.3.13; sqlfluff `test/fixtures/dialects` at master, 2,558 `.sql` files, and PostgreSQL `src/test/regress/sql`, 245 files, 5.15 MB):

| | sqruff | tree-sitter-sequel |
|---|---|---|
| Round-trip (raw segments == input) | 0 failures in all files (lossless) | not applicable (no trivia; our adapter rebuilds it) |
| Speed, dialect fixtures (0.01-0.25 MB per dialect) | 0.35-0.85 MB/s (ansi 0.43, postgres 0.48, tsql 0.72) | 1.1-2.5 MB/s (ansi, postgres, mysql, sqlite) |
| Speed, PostgreSQL regress (5.15 MB) | 1.00 MB/s | 2.15 MB/s |
| Files with `Unparsable` / `ERROR` on fixtures, own dialect | ansi 8/214, postgres 62/210, mysql 36/258, tsql 38/195, snowflake 40/277, bigquery 2/147, sqlite 15/49, databricks 43/93, duckdb 22/60 (all of these parse in sqlfluff) | ansi 79/214, postgres 183/210, mysql 214/258, sqlite 29/49 |
| Files with errors, PostgreSQL regress (postgres) | 185/245 (plus 11 with lexer errors, 1 panic) | 219/245 |
| Panics | 1 each in redshift, sqlite, starrocks, trino, postgres regress; `soql` hung over 10 minutes | none |
| Peak memory (whole run) | 9-22 MB per dialect, 79 MB on the 5 MB regress set | 10-20 MB |
| Binary added (stripped, panic=abort, vs an empty binary of 0.33 MB) | +9.4 MB (all dialects; per-dialect features not measured) | +13.4 MB |
| Crates added, licenses | 78 (35 MIT OR Apache-2.0, 18 Unicode-3.0 (ICU), 11 MIT, 7 Apache-2.0 OR MIT, 2 Apache-2.0, 2 Unlicense OR MIT, Zlib, BSL-1.0 OR Apache-2.0) | 2 (MIT, tree-sitter runtime shared) |

**Result:** the rule rejects sqruff (0.4-1.0 MB/s against 5, +9.4 MB against 4). tree-sitter-sequel is no fallback: it is no faster by an order of magnitude, adds 13.4 MB, and fails on 37% (ansi) to 87% (postgres) of the fixtures. Both parsers are rejected and the parser choice is open (candidates: a hand-written lexer and parser for the ansi/postgres core, sqruff with a faster front end, or relaxing the 5 MB/s and 4 MB limits); step 1 is not started.

## 1. Parser and tree
- **Pin:** `=0.N.M` of both sqruff crates in the root `Cargo.toml`, Dependabot-ignored like oxc and ruff; a bump touches the kind table and adapter only.
- **Adapter:** sqruff segments map 1:1 to nodes (sqlfluff's segment type names, e.g. `select_statement`, `join_clause`, `keyword`), its whitespace, newline and comment segments are trivia; Indent/Dedent meta segments are dropped (zero-width, layout is Sudsy's).
- **Errors:** an `Unparsable` segment is a syntax error at its span; the file is reported, not formatted or linted (standing decision).
- **Dialect:** option `dialect` (sqlfluff names: `ansi`, `postgres`, `mysql`, `tsql`, `bigquery`, `snowflake`, ...), default `ansi`; the importer reads sqlfluff's `dialect`.

## 2. Formatter
- Target: `sqlfluff fix` with only the LT and CP rules enabled, pinned sqlfluff version for parity; sqruff as a second check (speed comparison).
- Options (each named, presets set them): `line_width` (80), `indent_style`/`indent_size` (space, 4), `comma_position` (`trailing` | `leading`), `keyword_case`, `identifier_case`, `function_case`, `literal_case`, `type_case` (`consistent` | `upper` | `lower` | `capitalise` | `preserve`), `line_ending`.
- Embedding: SQL in Markdown fences (`sql`) and, later, SQL strings in other languages are not in scope.

## 3. Lint
- Catalog `docs/parity/sqlfluff.yaml` from sqlfluff's 79 rules (AL 10, AM 9, CP 5, CV 13, JJ 1, LT 15, RF 7, ST 12, TQ 4, Oracle/Postgres dialect rules); parity runner `scripts/parity.sh sqlfluff` against pinned sqlfluff.
- First: AM (ambiguous), RF01-RF03 (references), ST (structure), CV rules that catch bugs, AL rules that are not style. Zero-config: AM and the structure/reference bug rules on; capitalisation and aliasing style off.
- Pattern rules over sqlfluff's segment names where a rule is structural.

## 4. Parity and corpus
- Format: sqlfluff's dialect fixtures run through `sqlfluff fix --rules LT,CP` (per-dialect ratchet groups), PostgreSQL regression files (postgres dialect), and one real-world dbt-free SQL project with a checked license. Targets: 60% after step 2's first pass, 90% when step 2 closes; every difference named.
- Lint: the zero-config set on the same corpus, 0 unexplained mismatches.

## 5. Steps
| Step | Builds | Est. lines |
|---|---|---|
| 0 | Spike: parser decision with measured table (section 0) | (not committed) |
| 1 | `lang-sql`: dependency pin, adapter, lang.yaml (dialect option), provider and registration, verbatim formatter, bench input (cap 150 KB, 3 dialects), corpus fetch script and round-trip test, parse-speed numbers, Markdown `sql` fences, parity harness and catalog | 1,200 |
| 2 | Formatter to section 4's targets: layout, indentation, line breaking, capitalisation options, safety-check `value` hooks for re-cased keywords | 2,500 |
| 3 | Lint: zero-config set, first rule families, presets only where they set something | 1,500 |
| 4 | Migration: `sudsy init --from sqlfluff` / `--from sqruff` (`.sqlfluff`, `setup.cfg`, `tox.ini`, `pyproject.toml`, `.sqruff`; `exclude_rules`, `rules`, per-rule sections) | 400 |

Later (not this wave): templated SQL (Jinja/dbt rendering and mapping back, as sqlfluff's templaters); SQL embedded in other languages' strings; the soql dialect.
