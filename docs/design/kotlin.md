# Design: Kotlin

Goal: the formatter and linter Kotlin users switch to: ktlint's formatting (`ktlint_official` code style; `intellij_idea` and `android_studio` as options) and its standard rules. `.kt`, `.kts` and Gradle Kotlin DSL files. Built in the steps of section 3; this note is deleted when the last step lands.

## 1. Parser comparison (measured, not committed)
Linux arm64 Graviton, 8 cores, rustc 1.99.0, release with LTO; parse of the whole file from memory, warm, no adapter. Corpus: 5,564 `.kt` (30 MB) and 284 `.kts` of kotlinx.coroutines, ktor, nowinandroid, ktlint and the Kotlin repo's `libraries/stdlib` (pinned in `tests/conformance/fetch-kotlin-corpus.sh`); the Kotlin compiler's PSI test inputs (467 valid, 317 with errors, ground truth from the expected trees); 50 Kotlin 2.x probes; 441 mutants (one `)`, `}` or `{` deleted). Binary: stripped release with only the parser and a `main` (empty `main`: 0.33 MB).

| | ktrs-parser 0.5.1 | rezel-lang-kotlin 0.0.0 | brokk tree-sitter 0.4.9 | tree-sitter-kotlin-sg 0.4.1 | tree-sitter-kotlin-ng 1.1.0 |
|---|---|---|---|---|---|
| Kind | Rust, hand-written port of the compiler's PSI parser, flat lossless tree | Rust, LR (Lezer grammar), concrete tree without many punctuation tokens | C grammar (fwcd fork) | C grammar (fwcd fork) | C grammar |
| `.kt` / `.kts` files failing | 2 (BOM) / 0 | 50 / 282 (no script mode) | 24 / 0 | 102 / 0 | 216 / 0 |
| Valid PSI inputs rejected (of 467) / invalid detected (of 317) | 7 / 317 | 87 / 307 | 61 / 258 | 87 / 262 | 117 / 290 |
| Kotlin 2.x probes failing (of 50) | 2 | 4 | 1 | 12 | 17 |
| Mutants: share of the file inside error nodes (median / p90) | 0 / 1.3% | 0 / 0.4% | 1.2 / 94% | 2.9 / 95% | 0.7 / 90% |
| Parse speed | 53.8 MB/s | 16.1 | 7.6 | 7.6 | 4.6 |
| Stripped binary added | +0.26 MB | +0.39 MB | +10.8 MB | +5.9 MB | +3.5 MB |
| Upstream | one author, Sep 2026, MIT/Apache-2.0, ports Kotlin 2.4.20 | one author, one release | BrokkAi fork of fwcd | ast-grep fork | tree-sitter-grammars, last release Jan 2025 |

ktrs's tree is PSI-identical (node and token names of the Kotlin compiler, which ktlint's rules are written over), whitespace and comments are tokens, and its tokens tile the source exactly on every file tried. Limits found: it needs LF text (CRLF files fail), overflows the stack on deep nesting (brackets, prefix operators, `else if` chains: an abort), takes exponential time on nested annotation arguments (`@A(@A(...))`, 22 levels is 4 s) and is quadratic on very long binary chains.

**Decision (2026-11-16, the user): ktrs-parser through the adapter replay**, pinned exactly (`=0.5.1`, Dependabot-ignored; a bump touches the kind table and adapter only), with a CRLF offset map, a nesting guard before the parse, a large-stack parse thread and `catch_unwind`, `.kts` as a script. Only `ktrs-parser`, `ktrs-lexer` and `ktrs-syntax` are used; `ktrs-fmt` and `ktrs-lint` are not dependencies (layout and rules are ours) but are a behaviour reference.

## 2. Targets
- Formatter: safe (second format identical, no file refused, meaning preserved) and 85-90% of real-world files identical to `ktlint --format` in `ktlint_official`.
- Lint: ktlint's standard rules that are not pure formatting; the rest planned in `docs/parity/ktlint.yaml`.

## 3. Steps
| Step | Builds |
|---|---|
| 1 | `lang-kotlin`: parser adapter, lossless tree, provider, `.kt` / `.kts`, verbatim formatter, corpus script, bench inputs, docs |
| 2 | Formatter: ktlint layout, `scripts/parity.sh format ktlint`, code style options |
| 3 | Lint: ktlint catalog, rule families, default-on bug-catching rules |
