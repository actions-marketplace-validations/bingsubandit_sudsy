# Design: Gradle evidence

Measurements before any Gradle product code (method: [../development/no-reference-tool.md](../development/no-reference-tool.md), [../architecture/adding-a-language.md](../architecture/adding-a-language.md)). Scope: `*.gradle` (Groovy DSL: build, settings, init, script plugins); `*.gradle.kts` is Kotlin and covered. Scripts: `scripts/evidence_gradle*.py|sh`, `scripts/evidence_gradle_parsers/` (Rust harness). Looked up 2026-11-26. Machine: AWS Neoverse-V2 aarch64, 8 cores, rustc 1.99.0. Tools under the slot (`gradle-eval`, `scripts/evidence_gradle_tools.sh`): Temurin JDK 21.0.13, npm-groovy-lint 19.0.1, Gradle 9.9.2, Spotless plugin 8.12.0 (greclipse).

## Corpus
43 projects pinned by commit in `tests/conformance/fetch-gradle-corpus.sh`, listed before any result (known JVM, Android, plugin, Spring, Apache, Netflix, Elastic projects); a project joined if its tree has at least 4 `.gradle` files (of 86 looked up, 43 qualified). 6,481 `.gradle` files (54 MB); `scripts/evidence_gradle.py` caps a project at 60 files (first by path): 1,538 files (3.48 MB). Split by project, seed 20261204: 22 deciding, 21 held out (lists printed by `scripts/evidence.py gradle table`). Groovy's own parser (inside npm-groovy-lint) rejects 2 files (a `$.platforms` GString, a file starting `dependencies {`): the valid set is 1,536.

## Tools (usage evidence)
| Tool | Evidence | Verdict |
|---|---|---|
| Spotless `groovyGradle` + greclipse (Eclipse Groovy formatter) | `com.diffplug.spotless` in 18,048 `build.gradle` (GitHub code search), `groovyGradle` in 1,852, `greclipse` in 1,704; plugin 5.7k stars, pushed 2026-11-25; Apache-2.0; needs JVM and Gradle | primary for format (the languages.yaml estimate named npm-groovy-lint) |
| npm-groovy-lint 19.0.1 (MIT, wraps CodeNarc) | npm 179,900 a month, VS Code extension 491,782 installs; but `.groovylintrc.json` in 241 repositories, 47 workflow and 15 pre-commit hits; 246 stars, pushed 2026-11-25 | second format reference; lint only for generic Groovy |
| CodeNarc (Apache-2.0) | `codenarc` in 878 `build.gradle`; the Gradle plugin lints Groovy source sets, not build scripts | rule engine of npm-groovy-lint |
| Nebula gradle-lint plugin | 316 `build.gradle`, 793 stars; runs inside a build (needs the evaluated project) | partial source, not runnable offline |
| Android lint (Gradle detectors) | runs in every Android build; needs the SDK and a build | partial source (GradleDeprecated, GradleDependency) |
| IntelliJ formatter, prettier-plugin-groovy (npm 21k a month) | editor only; plugin not a reference | none |

## Format
No formatter is shared: on the 1,538 files npm-groovy-lint `--format` (its `format` preset, 4 spaces) changes 1,299 files (84%, 67,155 diff lines; it forces single quotes in 100% of plain strings), greclipse with `indentWithSpaces(4)` changes 1,007 (65%, 71,927 lines; its default indents with tabs), and the two outputs are equal on 322 files (21%). 28 files raised an error in greclipse (Groovy parses all but 2). Setting aside leading blanks, trailing blanks and CRLF, greclipse output equals the source on 1,103 files (71.7%; with blank lines 73.4%): it is mostly an indent normaliser; npm-groovy-lint 417 (527). Neither is held to the corpus style, so the format leg follows the decision table with greclipse as the comparison.

Decision table (`python3 -I scripts/evidence.py gradle table`; one vote per project over the 22 deciding projects; agreement is the share of the 21 held-out projects' occurrences with the chosen form):
| Row | Votes | Rule | Chosen | Held-out agreement |
|---|---|---|---|---|
| indent character | 22 space | default | space | 76.3% (tabs in hibernate-orm and others) |
| indent of a block body | 16 four, 6 two | default + option | 4 spaces, option width | 40.9% (large 2-space projects) |
| quotes of a plain string | 13 single, 9 double | as written | as written | n/a |
| dependency declaration parens | 15 none, 4 parens | default + option; a rewrite, left as written | as written | n/a |
| blank before `{` | 22 one | default | one | 99.4% |
| blanks around `=` | 22 both | default | both | 99.5% |
| blank after `:` in a named argument | 16 after only | default | after only | 88.5% |
| blank after a comma | 21 one | default | one | 98.1% |
| blank after `if/for/while/switch` | 21 one | default | one | 97.9% |
| `else` placement | 14 after `}`, 1 own line | default | after `}` | 100% |
| blanks inside a one-line closure | 18 inside | default | inside | 96.0% |
| blank after `//` | 21 one | default | one | 96.6% |
| blank lines in a row | 22 one | default | one or two kept | 88.2% |
| blank line after `{` | 22 none | default | none | 100% |
| trailing blanks of a file | 22 none | default | none | 99.1% |
| final newline | 22 yes | default | yes | 90.5% |
| line endings | 22 LF | default | LF | 97.8% |
| trailing semicolon | 22 none | default | none | 99.8% |
| continuation indent after a trailing comma | 18 same as previous line | loose measure | as written | n/a |
| line width | 94% of lines at most 80, 1% over 100 | no width evidence | as written | n/a |

Pooled held-out agreement over the decided rows: 87.1% (mean of the rows' project means 92.5%). Indent width is the only row under 80% pooled and it carries the 4-space default with an option. Each row is a measured count on masked code; a row's regex is approximate (the `:` and continuation rows are loose). Not done: the 30-file harm sample (no formatter), meaning oracle (Groovy's parser via npm-groovy-lint gives an AST check: usable, not run), stability (no formatter).

Recommendation: format leg per the no-reference path with greclipse as the comparison tool (its 4-space output is the reference for indent, wrapping and one-line closures), default 4 spaces, option indent width, quotes and parens as written; npm-groovy-lint is not the primary.

## Lint
No tool reads Gradle semantics offline, so the leg follows no-reference-tool.md section 4. npm-groovy-lint default config (CodeNarc `recommended`, 80 s over the corpus): 353 findings of 27 rules in 131 files: JavaIoPackageAccess 210 (EJB-spec rule, noise in build scripts), UnusedVariable 24 (error), UnusedImport 24, CatchException 13, GStringExpressionWithinString 12 (a real bug: `'${x}'`), EmptyMethod 11, others 1 to 7 each; no finding is Gradle-specific. Real and cheap: UnusedImport, UnusedVariable, GStringExpressionWithinString, GStringAsMapKey, DeadCode, EmptyIfStatement, BitwiseOperatorInConditional, ConstantAssertExpression (about 70 findings); the rest is taste or noise (sample of 2 per rule read).

Checklist candidates from the Gradle upgrade guides and the Android plugin API, yield on the corpus (`scripts/evidence_gradle_checks.py`; occurrences / files / projects): `apply plugin:` 869 / 388 / 34; Android `compileSdkVersion` family 481 / 176 / 10; `sourceCompatibility` outside `java {}` 266 / 111 / 25; `buildDir` 145 / 94 / 24; `buildToolsVersion` 136 / 136 / 6; dynamic versions 48 / 30 / 7; `lintOptions` 56 / 56 / 7; removed `compile`/`testCompile` 24 / 11 / 4; `jcenter()` 23 / 12 / 2; `tasks.create` 22 / 19 / 7; archive `baseName`/`classifier` 16 / 14 / 5; `mainClassName` 10 / 8 / 4; `uploadArchives` 1; http:// repository and `<<` tasks 0. Some need the Gradle version and are style (apply plugin, tasks.create). The checklist is not yet reviewed with sources and examples (lead checkpoint), the precision sample is not run.

Recommendation: lint leg with no reference tool: the checklist above plus the 8 CodeNarc rules that read the file; Android lint and Nebula named as partial sources, left out for needing a build.

## Parser candidates
Whole files from memory, warm, best of 5, `scripts/evidence_gradle_parsers.sh`, 1,538 files (3.48 MB), tree-sitter 0.27.2, parser.c at -O2; 70 probes of Gradle and Groovy forms (`probes.groovy`). Binary size is the stripped harness over the lexer-only harness.
| | dekobon 0.6.0 | murtaza64 (2a6ddd5) | loganprice (b217b73) | amaanq 0.1.2 | hand-written (lexer prototype) |
|---|---|---|---|---|---|
| Kind | C, generated, untyped, external scanner | C, generated, untyped | C, generated | C, generated | Rust, 143-line lexer on `core_syntax` style tokens; parser not written |
| Files without error node | 1,374 (89.3%) | 1,240 (80.6%) | 1,106 (71.9%) | 710 (46.2%) | lexer: 1,538 tile, 0 unknown tokens, all brackets balanced |
| Probes failing (of 70) | 4 | 5 | 6 | 23 | not parsed |
| Speed | 18.7 MB/s | 7.0 MB/s | 15.6 MB/s | 8.8 MB/s | lexer 218.8 MB/s |
| Binary added | 1,536 KB | 960 KB | 640 KB | 1,099 KB | 0 |
| Error-free files whose leaves plus blanks do not tile | 41 (slashy-string slashes, `\` continuation) | 481 (block comments) | 0 | 145 of all | 0 |
| Mutants: one deleted bracket, error share median / p75 / p90 (250 files) | 0.1 / 52.3 / 100% (21 no error) | 0.3 / 95.9 / 100% (28) | 0.3 / 41.6 / 100% (34) | 0.1 / 20.3 / 100% (52) | n/a |
| Deep nesting 100,000 `foo(` and `foo {` | no error, 100 ms | no error, 240 / 400 ms | no error, 110 ms | no error, 110 / 160 ms | n/a |
| Upstream | Apache-2.0 OR MIT; created 2026-05; 8 releases in 6 months; 5 contributors, 100 commits in the year; 4 stars; crates.io 140k downloads; rust-version 1.90 | MIT; 26 stars, 11 contributors, pushed 2026-09; no crate | MIT; created 2026-10, 0 stars, 2 contributors | no license field; one contributor, last push 2024-11 | ours |
| Maintainers and funding | one active maintainer (fork of the grammar), unpaid as far as visible | spare time, 35 commits in the year | one person, new | none | us |
Decodetalkers ships no generated `parser.c` (not built); `arborium-groovy` is a bundle of a grammar above (not measured). Not measured: memory, an adapter, the time to build parser.c (6 to 8 MB of C each).

dekobon, first error of the 164 files with an error node: `,` 67 and `:` 33 (a command expression with several arguments or named arguments after a dotted name: `inputs.property "k", v`, `dependsOn a, b`, `proguardFiles f, 'x'`, `attributes 'k': v`), `->` 6, the rest single causes (not root-caused one by one). Failing probes: a multi-argument command call after `release {` or `attributes`, `foo.bar baz, qux: 1`, `[*a, *: m]`. These command expressions are the core Gradle idiom, so an own fork of the grammar must fix them; the fix is not measured.
Hand-written: lexer prototype 143 lines (strings with nested `${}`, triple quotes, slashy and dollar-slashy by the previous token); tiling, balance and speed are measured, the parser is not: estimate 1,000 to 1,400 lines for statements, command expressions by juxtaposition, closures, GString re-parse, newline rules (Batchfile: 1,216 lines in all), plus the formatter on shared mechanisms, against the 3,000-line budget.

Recommendation: the dekobon grammar, copied into `grammars/tree-sitter-groovy` and fixed for the command-expression causes (89.3% clean now, 18.7 MB/s, Apache-2.0 OR MIT, the only candidate with active upstream and a scanner for GStrings and slashy strings; the adapter must add the slashy delimiters and `\` that are not leaves in 41 files). Alternative if the user wants no C parser and no maintainer risk: hand-written (+0.3 in the cost: parser 1.5 against 1.2), with a lexer already shown. The choice is the user's.

## Not verified
Whether the corpus is typical of Kotlin-era repositories that moved to `.kts` (only Groovy files count); greclipse with a project `.eclipse-groovy` properties file (default only); npm-groovy-lint `format` with a custom `.groovylintrc`; slashy-string tokens of the lexer on code with `/` after an identifier in a command expression (135 found, all brackets balanced, not read); first-error causes beyond the top ones; harm sample, stability, meaning oracle, checklist review and precision.
