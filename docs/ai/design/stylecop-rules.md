# StyleCop rules marked `sdk`

Sudsy implements the StyleCop rules that [../parity/stylecop.yaml](../parity/stylecop.yaml) marks `sdk` (a .NET SDK diagnostic did them; user decision). Goal: the lint of the `csharp/stylecop` preset reports at least 85% of the findings of the 175 default-on StyleCop ids on the C# corpus (7,659 files), same line. Standing: 100,884 of 409,320 findings (24.6%, documentation comments parsed). The target needs 347,922, so +247,038 more.

This note is step 1 (the mapping table, before code). Rules are shared mechanisms plus rows and plain options; `lang-csharp` holds only C# facts (kinds, tags, default visibility, the worker check). Every rule is off by default and on in the preset, except where the table says otherwise. Counts are missed findings of the last parity run (`docs/ai/parity/queue.md`).

## Mapping table

| Sudsy rule | StyleCop ids (missed) | Shared concept (index entry) | C# facts (data) | Worker |
|---|---|---|---|---|
| `qualify-instance-members` (new) | SA1101 (108,918) | new `semantic-finding` family: a rule reports the places of one `Finding.kind` the provider's semantic layer returns; the word to insert (`this`) is a plain option | worker check `unqualified_member`: an `IdentifierName` that binds to a non-static field, property, method or event of the containing type, not after `.`, not in `nameof`, not a local or parameter | yes (`symbol_findings`, one more kind of the existing walk); without the worker the rule reports nothing |
| `missing-doc-comment` rows for C# | SA1600 (59,930), SA1602 (7,380), SA1601, SA1604-SA1606 stay `doc-*` | index entry `missing-doc-comment` (core-rules/doc_comments.rs; Java uses it): rows of declaration kinds, `visible` modifiers, `implicit-public-in` | `types`, `members`, `enum-items` rows; visible = `public protected internal` (StyleCop default: private members and fields are exempt); interface and enum members are visible without a modifier; partial types report once | no |
| `doc-param-missing` (new) | SA1611 (45,566), SA1612, SA1618, SA1620 | the parameter model of the doc comment, moved from `lang-csharp/doc_comments.rs` (SINGLE-USE) to `core-rules/doc_comments.rs` as the second user (Java `@param`, Checkstyle JavadocMethod) arrives: option `syntax: xml\|javadoc`, rows for the parameter and type-parameter lists | `<param name>`, `<typeparam name>`; a comment with `<inheritdoc/>` is skipped | no |
| `using-placement` (new, `imports` family) | SA1200 (25,732) | index entry `import-order` (core-rules/imports): option `placement: inside\|outside` for a directive list relative to a namespace body; report only (moving a `using` changes name lookup), no fix | `using` directives, block and file-scoped namespace kinds; a file-scoped namespace counts as `outside` the way StyleCop reads it | no |
| `naming-convention` rows for C# | SA1309 (9,589), SA1300-SA1308, SA1310-SA1314 | index entry `naming-convention` (core-rules/naming.rs; Java, Kotlin, GraphQL use it): rows `field&private`, `const`, `static&readonly`, parameter, type parameter with a convention or a regex | `_` prefix and `m_`/`s_` are the options of the rows (`match: regex`); all rows are data of `rules/naming.yaml` | no |
| `file-header` (new) | SA1633 (7,626) | index entry for a first-comment check; option `required` plus `pattern` (empty: any comment) shared with Java Checkstyle Header and RegexpHeader as second user | `//` lines and `/* */` as the header; the `<copyright>` forms SA1634-SA1641 stay `sdk` (they need a stylecop.json) | no |
| `curly-braces` rows for C# | SA1503 (6,318), SA1519, SA1520 | index entry `curly-braces` (core-rules/curly.rs; Java and Kotlin) | rows for `if`, `else`, `for`, `foreach`, `while`, `do`; a nested `if` as the body of `else` (`else if`) stays | no |
| `unnecessary-parentheses` | SA1119 (3,220) | reads the needs-parens decision of `parenthesized-expression` (core-format/shapes/parens.rs); lint over the same predicate, fix removes the pair | parenthesized kinds, the expression kinds that never need them (`return (x);`, `(x)` as an argument, `((a))`) | no |
| formatter | SA1009 (6,637) | a layout finding, not a rule: the fix is the preset (`csharp/stylecop` writes the spacing StyleCop wants); the missed count is the formatter's difference in the parity run | none | no |
| left as is | SA1623 (3,892: `bool` alias in the documentation), SA1316 (3,089, planned) | gaps named in the catalog | | |

Not started: SA1121, SA1142, SA1205-SA1207, SA1302, SA1400-SA1408, SA1401, SA1637-SA1641 and SX1101/SX1309 (the opposites, off by default); each is under 1,500 missed findings. A rule whose catalog line changes from `sdk` is one that exists; the rest stay `sdk`.

## Points settled

- **SA1101**. The worker answers with the span of each unqualified instance reference (file-wide, with the other symbol findings; no point queries), so the cost is one more kind in a walk that already runs under `standard` depth. At `fast` depth the rule does nothing, as every worker rule. Measured before the rule is on in a preset: files resolved and milliseconds per file on `bench/inputs/cs` and on the corpus, in `docs/ai/performance/numbers.md`. A syntax-only version (members of the enclosing type in the file) would miss inherited and partial members and is not built.
- **Doc comments**. `missing-doc-comment` and the `@param` check are one family each in `core-rules`; the C# doc model keeps what is C#: the XML element names and `inheritdoc`. Java and Kotlin keep their output (their corpora before and after, byte-identical).
- **SA1200, SA1633, SA1503, SA1119**. All four are lint rules. None is a layout finding the formatter writes under the preset (the formatter never moves a `using`, adds a header or adds braces), so the formatter keeps its option set; the lint rule has no fix except `unnecessary-parentheses` and `curly-braces` (safe).
- **Budget**. New default-on rules in the preset only; lint instructions of C# up to +40%; every other language within +1%.

## Reach

SA1101 + SA1600 + SA1611 + SA1200 = 240,146 of the 247,038 needed. SA1309, SA1633, SA1602, SA1503 and SA1119 add 34,333, so the set reaches 274,479 more findings (91.6% if every one matches on the same line). The margin covers findings the mapped rule reports on another line; the first run of the set decides what is cut or sharpened.

## Steps

1. This note, sent to the lead before code.
2. One landing: the worker check and `semantic-finding`; the `missing-doc-comment`, `naming-convention` and `curly-braces` rows; `doc-param-missing`, `using-placement`, `file-header`, `unnecessary-parentheses`; the preset rows; fixtures per rule; the catalog lines (`sdk` becomes `rule <id>`), the `sdk` row of [../parity/README.md](../parity/README.md#format) and the `builtin.md` line saying SDK style rules are not reimplemented.
