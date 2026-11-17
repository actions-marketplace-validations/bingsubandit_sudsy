# Design: moving the older languages onto the shared core

Goal: JS/TS, Python, HTML and its dialects, CSS/SCSS/Less, SQL, C#, YAML, Markdown, Java, shell and the rest stop carrying their own copies of what the shared core already has or should have, and rules that check the same thing share one id and one implementation. The method of [shared-core.md](shared-core.md) holds: roles from the grammar, kits in `core-format`, one binder and one flow builder, pattern rules and families with data rows, policy in `lang.yaml`. This note is the plan, not the work. It is deleted when its last step lands; each step updates the docs of the area it changes.

## 1. Where the lines are (main at `f9f98ec3`)
`scripts/cost.sh --languages`: shared crates 30,971 lines, `lang-*` 78,742, shared share 28.2%. Formatter 33.0k, rules 23.2k.

- **Ids are already mostly one per thing.** 722 rule ids, 647 released in 0.0.2. 94 ids cover `css`+`less`+`scss` and 128 cover the four HTML dialects (one crate, one id). Cross-language ids exist today: `no-self-compare`, `no-constant-condition`, `no-constant-comparison`, `no-assignment-in-condition`, `no-empty-block` (shared pattern files, K1), `no-duplicate-case`, `no-duplicate-keys`, `no-empty-keys`, `key-name-casing`, `sorted-*`, `max-line-length`, `comment-space`, `no-empty-comment` (shared Rust), `unused-import`, `unused-local-variable`, `naming-convention`, `unused-variable`, and the YAML set shared with GitHub Actions. What is left to merge is 36 ids that name the same finding in several languages (section 3), plus code that is the same under different ids.
- **Most rule lines are real per-language knowledge.** About 200 Rust checks average 30 lines each (Python `control.rs`: `try_except_pass` 23, `blind_except` 31; Ruby `clauses.rs`: `rescue_exception` 13, `suppressed_exception` 26; JS `misc.rs`: `self_assign` 15). Moving a check to a role rule saves its 15-50 lines, not a file. The large blocks are: the scope rules of JS and Python (about 3,100 lines with their tables), CSS's name families (794), Python's ruff families that exist in no other language (`isort` 712, `%`-format 579, pyupgrade 417), C# documentation rules (about 1,950), JS regex rules (307).
- **Formatters are the bigger target.** Python uses no shape of `core_format::shapes`; JS keeps call arguments, objects, parameters, statements, `needs_parens` and TS types (about 4,400 of its 7,959 lines) as its own code; C# keeps lists, braces and sequences (1,425 of 3,327); CSS, YAML, Markdown, HTML, SQL and shell are on the document printer but on no shape.

Estimate for the whole plan (lines of hand-written Rust, from the measured files, the shared lines the plan adds counted): lint about -2,800 in `lang-*` and +1,300 shared; formatter about -4,500 in `lang-*` and +2,000 shared. Net about -4,000 lines and the shared share from 28.2% to about 32%. Rows below say which part of that each step carries. Every number is an estimate to be replaced by the step's own `scripts/cost.sh` output; a step that misses its estimate by half stops and reports.

## 2. Inventory
`scripts/rule_inventory.py` (re-runnable, reads `docs/rules/rules.json` and the rule files; `--list` one row per rule and language with kind, default and upstream ids, `--shared` ids that exist in several languages). Kinds: *shared* (pattern file in `core-patterns/shared`), *pattern* (`rule:` file), *family* (rule file naming Rust code through `family:` and `params:`), *rust* (no rule file). Summary, rows are (id, language):

| Language | ids | shared | pattern | family | rust | on by default | Rust lines | rule-file lines |
|---|---|---|---|---|---|---|---|---|
| python | 129 | 4 | 11 | 113 | 1 | 111 | 6,362 | 1,139 |
| html (+vue, svelte, angular) | 128 | 0 | 39 | 76 | 13 | 71 | 326 | 1,160 |
| css (+scss, less) | 92 | 0 | 14 | 78 | 0 | 14 | 2,255 | 926 |
| js (+ts) | 73 | 5 | 22 | 45 | 1 | 38 | 4,223 | 628 |
| csharp | 67 | 5 | 5 | 18 | 39 | 7 | 4,310 | 225 |
| dockerfile | 47 | 0 | 14 | 27 | 6 | 19 | 616 | 428 |
| ruby | 37 | 5 | 1 | 26 | 5 | 37 | 1,500 | 248 |
| shell, github-actions, protobuf | 24, 21, 21 | 0 | 22, 3, 21 | 0 | 2, 18, 0 | 11, 11, 1 | 126, 0, 5 | 229, 0, 217 |
| java, markdown, kotlin, powershell | 19, 16, 14, 13 | 5, 0, 5, 0 | 9, 13, 7, 13 | 1, 0, 1, 0 | 4, 3, 1, 0 | 13, 3, 9, 8 | 310, 92, 6, 146 | 124, 145, 72, 123 |
| yaml, sql, hcl, php, graphql, makefile, gitignore, editorconfig, xml, ini, json, `all` | 12, 12, 11, 7, 6, 5, 5, 3, 2, 1, 1, 9 | 5 in php | 3, 6, 6, 0, 1, 0, 2, 1, 1, 1, 0, 0 | 0 (graphql 2) | 9, 6, 5, 2, 3, 5, 3, 2, 1, 0, 1, 9 | 3, 8, 3, 7, 2, 1, 3, 3, 1, 0, 0, 2 | 582, 190, 262, 6, 229, 218, 157, 253, 220, 5, 89, 0 | 41, 60, 61, 0, 27, 0, 22, 13, 18, 10, 0, 0 |

(*Family* rows are the cheap kind: one 8-12 line data row each; their Rust is the family. The `rust` column of C#, GitHub Actions and the shared `all` set are ids implemented in Rust without a rule file.)

## 3. Clusters
A cluster is rules that check the same thing. Two kinds: **A. one id** (the ids are merged, the new id has a row per language) and **B. one implementation, ids stay** (the tool-specific ids are kept as rows of a shared family, because their options, defaults or reference-tool ids differ and nothing would be gained by merging). Lines are the estimate of section 1: `lang` removed from `lang-*`, `shared` added. Catalog equivalents are the `docs/parity/*.yaml` entries whose `rule <id>` changes.

### A. Same finding, one id
Default levels stay per (id, language) row; no row changes what is on by default.

| New id | Replaces (bold: in 0.0.2) | Languages | Mechanism | lang / shared | Catalogs |
|---|---|---|---|---|---|
| `unused-import` | `remove-unused-usings` (C#, **R**) | C#, Java, Kotlin, Python, JS | scope family; C# keeps its worker-backed check under the shared id | 0 | stylecop, roslynator IDE0005, checkstyle, ktlint |
| `unused-variable` | **`no-unused-vars`**, **`unused-local-variable`** (Java, Ruby) | JS, TS, Python, Java, Ruby, Kotlin, PHP, C# | scope family; ESLint's `args`, `caughtErrors`, `ignoreRestSiblings`, `varsIgnorePattern` become options; TS type reading stays a JS hook | JS -320, Python -150 / +150 | eslint, typescript-eslint, ruff F841, rubocop UselessAssignment, checkstyle |
| `unused-parameter` | `unused-method-argument`, `unused-block-argument` (Ruby, new) and the `args` part of `no-unused-vars` | Ruby, JS, Python, Java, Kotlin | scope family (exists for Ruby) | in the row above | rubocop, eslint |
| `undefined-variable` | **`no-undef`**, **`undefined-name`** | JS, Python, PHP (exists), Ruby | scope family: a read that no scope binds; the language's globals are a data list in its table (JS `BUILTINS` 75 lines, Python 270 move out of Rust); off where undecidable (Java, Kotlin: wildcard imports, inheritance) | JS -200, Python -250 / +200 | eslint, ruff F821/F822, phpstan |
| `use-before-define` | **`no-use-before-define`**, **`undefined-local`** (F823) | JS, Python, Ruby, PHP | scope family with positions (Python's `binding.rs` order logic moves into the binder) | JS -100, Python -200 / +120 | eslint, ruff F823 |
| `redeclared-variable` | **`no-redeclare`**, **`redefined-while-unused`** (F811) | JS, Python, C# (CS0128 is the compiler's) | scope family, option `only_if_unused` (default on for Python's table row) | JS -60, Python -100 / +80 | eslint, ruff F811 |
| `shadowed-variable` | **`no-shadow`**, `redefined-argument-from-local`, **`import-shadowed-by-loop-var`** | JS, Python, Java, Kotlin, PHP | scope family, option `kinds` | JS -130, Python -50 / +100 | eslint, ruff F402/PLR1704, checkstyle HiddenField (planned) |
| `unreachable-code` | **`no-unreachable`** | JS, TS, Python, Java, Kotlin, PHP, Ruby, C# (exists) | flow family on `core_flow::build` (JS `rules/reach.rs` moves to `core-rules`); C#'s syntactic fallback is deleted | C# -80, JS 0 / +0 (moved) | eslint, ruff (preview) |
| `no-fallthrough` | (JS only today) | JS, Java, PHP, Kotlin (`when` has none) | same family, `case_ends` | 0 | eslint, checkstyle FallThrough (planned) |
| `no-self-assign` | `self-assignment` (Ruby, new) | JS, Ruby, Python, Kotlin, PHP | shared pattern file: `role: assignment`, `same: [left, right]` | JS -15, Ruby -40 / +10 | eslint, rubocop |
| `no-empty-catch` | `try-except-pass` (new), `suppressed-exception` (new), **`empty-catch-block`** (PowerShell) | Python, Ruby, PowerShell, Java, Kotlin, PHP, C#, JS | shared Rust rule on `#catch` bodies; a language hook says which statements are no-ops (`pass`, `...`); option `broad_only` (ruff S110); `no-empty-block` stops reporting catch bodies | Python -25, Ruby -30 / +60 | ruff S110, rubocop, psscriptanalyzer, checkstyle EmptyCatchBlock |
| `no-broad-catch` | **`bare-except`**, `blind-except`, `rescue-exception` (new) | Python, Ruby, Java, Kotlin, PHP | role rule; the broad type names are a data option per language | Python -60, Ruby -13 / +40 | ruff E722/BLE001, rubocop, checkstyle IllegalCatch |
| `no-throw-generic` | `raise-vanilla-class`, `raise-exception`, `inherit-exception` (new) | Python, Ruby, Java, Kotlin | role rule on `#throw` / class heritage; type names as data | Python -10, Ruby -40 / +40 | ruff TRY002, rubocop |
| `no-useless-expression` | **`no-unused-expressions`**, `useless-expression` (new) | JS, Python, Ruby, PHP, Kotlin | shared rule on a new role `#expression-statement` | JS -50, Python -18 / +40 | eslint, ruff B018 |
| `boolean-literal-comparison` | **`true-false-comparison`** (Python) | Java (exists), Python, JS, Kotlin, C# | shared pattern file on `#binary` equality with a boolean `#literal` | Python -12 (yaml) / +10 | ruff E712, checkstyle |
| `no-debugger` | `leftover-debugger` (new) | JS (exists), Python, C# | banned statement or call rows | Python -24 | eslint, ruff T100 |
| `no-print` | **`print`**, **`pprint`**, **`no-write-host`** | Python, PowerShell, JS (`console.*`), Java, C# | banned-call rows with a per-language default `calls` option (the pattern engine's `in-option`) | 0 (rows) | ruff T201/T203, psscriptanalyzer |
| `no-eval` | `exec-call` (new), **`no-invoke-expression`** | Python, PowerShell, JS, PHP, Ruby | banned-call rows | 0 | ruff S102, psscriptanalyzer, eslint |
| `no-wildcard-imports` | **`avoid-star-import`** (Java), **`undefined-local-with-import-star`** (F403) | Java, Kotlin (exists), Python, C# | shared pattern file on `#import` with a wildcard | Java -5 / +10 | checkstyle, ktlint, ruff F403 |
| `no-trailing-whitespace` | (HTML only today) | all | shared line rule beside `max-line-length` (`core-rules/lines.rs`) | HTML -20 / +30 | html-validate, yamllint, markdownlint MD009 |

That is 18 merged ids (36 old ids replaced: 22 released, 14 new since 0.0.2) plus 2 widened ones. CSS has three copies of shared ids: `comment-no-empty` is `no-empty-comment`, `block-no-empty` is `no-empty-block`, `comment-whitespace-inside` is `comment-space` for block comments (3 released ids, about -50 rule-file lines); they go in the K1 step.

### B. Same implementation, ids stay
| Family | Rows (ids that stay) | Languages | Mechanism | lang / shared | Notes |
|---|---|---|---|---|---|
| `names` (allowed list, disallowed list, pattern, case, unknown, deprecated, vendor prefix of the name of a kind of thing) | stylelint's 60-odd name rules, html-validate `attr-pattern`, `class-pattern`, `id-pattern`, `name-pattern`, `attr-case`, `element-case`, HTML `deprecated` | CSS, HTML, XML, Markdown | `lang-css/src/rules/{names,table}.rs` (794 lines) move to `core-rules` with the language supplying only the `Target` (what the names of a kind are); the data tables stay language data | CSS -500, HTML -60 / +450 | ids unchanged; saves rows only where two ids were one rule with a different target |
| `naming-convention` | `class-naming`, `enum-entry-name-case`, `package-name` (Kotlin), 10 protobuf `*-case`/`*-suffix` ids, `async-suffix` (C#), `naming-convention` (Java, GraphQL, HCL) | Java, Kotlin, Protobuf, C#, GraphQL, HCL, Python (`ambiguous-*-name`) | one family over `#declaration` roles and a convention per kind; GraphQL's `naming_convention.rs` (98 lines) becomes the shared code | lang rule files -250 lines, Rust -100 / +100 | buf and ktlint ids stay as rows because presets and `init --from` name them |
| `unique-names` (exists) | `no-dupe-class-members`, `no-duplicate-enum-values`, `duplicate-stage-name`, `unique-targets`, `duplicate-table-alias`, `no-duplicate-heading`, `no-duplicate-pattern`, `no-duplicate-include`, `duplicate-ids` | JS, Dockerfile, Makefile, SQL, Markdown, gitignore, XML, GitHub Actions | rows; identity (case-insensitive, getter and setter) is `RuleHooks` | -250 / +40 | |
| `max` | `max-nesting-depth` (CSS), `selection-set-depth` (GraphQL), `max-body-length` (Makefile), later Checkstyle's | CSS, GraphQL, Makefile, Java | `lang-css/src/rules/max.rs` (142) to `core-rules`, counting a role | -100 / +120 | low value alone; do with the names family |
| `sorted-array-values` | `unsorted-dunder-all`, `unsorted-dunder-slots` | Python | a row of the shared rule with a target (a named assignment) | -100 | |
| doc comments | C#'s 24 `doc-*` rules (`doc-element` family, `doc_comments.rs` 661 lines) | C#, Java, Kotlin, Python, JS | a language-neutral doc model (list of tagged elements with text and name) from a per-language parser of the comment: XML for C#, Javadoc/KDoc/JSDoc tags, docstring sections | 0 now | **trigger**: the first Javadoc rule (Checkstyle `MissingJavadocMethod` is 18,283 missing findings). Do it then, not before: no second consumer exists yet |
| regex source | JS `no-control-regex`, `no-regex-spaces`, `no-empty-character-class`, `no-misleading-character-class`, `no-useless-escape` (regexp.rs + escapes.rs 638) | JS, Python, Ruby, Java, PHP | a regex analyzer in `core-rules` | 0 now | **trigger**: a second language asks for one of these (none in a default set today) |
| import order | `unsorted-imports` (isort 712 lines), `using-order` + the C# using sort, Kotlin's import ordering in `rewrite.rs` | Python, C#, Kotlin, Java | groups and ordering as policy data | maybe -250 | **no step**: isort's algorithm is isort-specific; revisit when Java's import rules land |
| member order | `member-order`, `accessor-order` (532 lines) | C#, Java | role table | 0 now | trigger: Checkstyle `DeclarationOrder` |
| todo comments, final newline, max blank lines | none exist | all | shared text rules next to `max-line-length` | - | not speculative: add when a catalog entry or the coverage run needs them (Checkstyle `TodoComment` 1,498 missing findings); formatter options own final newline and blank lines |

### C. Language-specific, and why
722 ids; 120 are in sections A and B. The other 602 keep their ids and code: 6 (K1) and 19 (`all`, YAML∩GitHub Actions) of them are already shared, leaving about 575 that exist in one language only. They are specific because the finding depends on a rule of that language or tool, not on a role:
- HTML family 128: element content models, ARIA, Vue/Svelte/Angular directives; Python 93: ruff's pyupgrade/bugbear/format-string/isort/`__future__`/typing checks (`%` and `str.format` strings 579 lines, type annotations 417); CSS 91: stylelint's property and value knowledge; Dockerfile 46 and shell 24: commands inside a shell string; C# 31: Roslyn bindings, StyleCop layout and `///` rules outside the doc family; JS 54: TypeScript-only and ECMAScript semantics (`use-isnan`, `valid-typeof`, `getter-return`, `require-yield`, 22 typescript-eslint rules, regex); Ruby 23: RuboCop Lint classes and calls; GitHub Actions 8, HCL 10, PowerShell 10, SQL 10, Markdown 15, Java 8, Protobuf 8, Kotlin 3, Makefile 3, gitignore 4, EditorConfig 3, GraphQL 2, XML, INI, JSON 1 each.
- A language-specific rule still moves to a pattern file or a family row when its check is a data row; that is a cost of the step that touches the language, not a cluster.

## 4. Rule ids change
The API is unstable (user decision): renamed and merged ids simply change. No aliases, no deprecation warnings. The changelog lists every renamed id (36 old ids replaced by 18 new ones; 22 were in 0.0.2). In the same step as each merge: rename the rule file, fixture folder and catalog entries, update presets, `sudsy init --from` mappings (`crates/cli/src/init/`, which debug builds check against the catalog), docs and the generated tables. `cargo xtask rename-rule <old> <new>` (about 120 lines) does the mechanical part and is the first lint step. A shared option that replaces a language option is renamed the same way, presets rewritten in the same step (presets stay pure data). The catalog needs one addition: an entry may name several ids (`no-unused-vars` maps to `unused-variable`, `unused-parameter`, `unused-import`).

## 5. Formatter migration
Rules from [shared-core.md](shared-core.md) hold: a ratchet (`tests/parity/*/differences.yaml`: files identical, second-format, refused) may only improve during a step; decision 1's 2-point allowance was for the pilot and is **not** assumed here (the lead may grant it per step). Presets and policy: a difference between tools is a named option in `lang.yaml` (declared once in `core_config::shared` when two languages have it), never a branch on a language or preset. Bench: a language's format up to +5%, every other language and metric +1% (CLAUDE.md); L1-L2 showed role-driven layout costs 2-6% where `Grammar::slot` is read, so JS and Java steps carry that risk.

Kit additions, each justified by 3 or more consumers, none by speculation:

| Addition | What | Consumers | Notes |
|---|---|---|---|
| `core_format::literals` | string quote switching (fewest escapes, `quote_style`), number spelling (case, trailing zeros, exponent), comment text (`#x` to `# x`) | JS `literal.rs` 266, Python `literal.rs` 374 + comment, CSS `text.rs` 189, YAML `scalars.rs` quotes | text in, text out: no layout risk |
| list layouts | `PreserveFirstBreak` (an object stays expanded when the source breaks after `{`), `MagicTrailingComma`, `Fill` (number arrays) on `shapes::delimited_list` | JS `object.rs` 550, Python `list.rs` 401, C# `lists.rs` 402, Java array and annotation lists, CSS value lists | Kotlin's `PreserveItemBreak` is the first of these |
| `shapes::signature` | parameter list + return type + type parameters, hugging a sole object or destructured parameter, break order | JS `decl.rs`/`arrow.rs` params (~800), TS signatures, Java `decl.rs` parameters, Kotlin (ktlint `function-signature`, 30,800 missing findings), C#, Python `def` | |
| `shapes::compound` | keyword, head, body, continuation clauses (`else if`, `else`, `catch`, `finally`, `while` of `do`), brace body or offside body (`BlockStyle::Offside`) | JS `stmt.rs` 407, C# `braces.rs` 512, Java `stmt.rs` 223, Kotlin, PHP, Python `compound.rs` 348 + `block.rs` 263, YAML block mappings and sequences (offside bodies) | the largest repeated construct left after chains; offside bodies have two consumers now (Python, YAML), so they are kit code, not Python code |
| `shapes::maybe_parenthesize` | `group(if_break("("), indent(softline, x), softline, if_break(")"))` with invisible parentheses | Python `paren.rs` 589 (ruff's `maybe_parenthesize`), JS `return`/`throw`/arrow body, C# `paren_group` | an extension of `shapes::parens`; three consumers, so it lands in F3, not with Python |
| call arguments | the hug candidates builder (flat, hugged, all broken out) with a `can_hug` hook | JS `call.rs` 713, C# `lists.rs` hook, Java, Kotlin, PHP | L1 found three languages share only the last step; attempt only after the rows above, and stop if fewer than 4 languages use it |

| Language | Formatter now | Moves to the kit | Stays language code | lang / shared | Ratchets that must hold | Bench risk |
|---|---|---|---|---|---|---|
| JS/TS | 7,959 | objects and arrays (`object.rs`), params, statements, literals, TS unions and type-argument lists, call arguments (last), the `needs_parens` table where it is the `precedence` data | JSX (662), Prettier's assignment and call predicates, `rewrite.rs` (481), TS declarations | -1,500 / +500 | prettier-js tests 1110 and real-world 2287 files identical, second-format 0+0, refused 0; angular, vue and svelte (they embed JS) | high: hot path, +5% budget |
| Python | 3,906 | `list.rs` 401 (lists, calls, params), `op.rs` 359 (chains: `binary_chain`, `conditional`), `paren.rs` 589 (`maybe_parenthesize`), `block.rs`/`stmt.rs`/`compound.rs` 863 (`compound` with offside body, `statement_sequence`), attribute chains (`call_chain`, ruff's fluent rule is `member_chain_min_calls` 2) | none of strings, docstrings, comments or blank lines: strings go to `core_format::literals` (F1; prefixes and triple quotes are literal policy data), comment placement to `core_format::comments` (`comment.rs` 233), blank lines to `shapes::statement_sequence` with blank-line counts per nesting level as a shared option (ruff's 2 around top-level definitions, 1 in a class; C#, Java, Kotlin have the same kind of setting), docstring indentation to a shared doc-text re-indent (second consumer of Kotlin's `#indentable-doc`; JSDoc, Javadoc and C# `///` follow) (`docstring.rs` 114); what stays is `del`/`return`/`assert` forms | -1,300 / +400 | ruff black-tests 189, ruff-fixtures 104, real-world 1006 identical, second-format 0 in all three | high: ruff parity is the tightest ratchet; python format +5% |
| C# | 3,327 | `lists.rs` and `braces.rs` onto list layouts and `compound`, the remaining binary and chain glue | `seq.rs` (separator engine), `flat.rs`, `branches.rs`, `query.rs`, options (preset-facing) | -500 / +200 | C# differential 5488 of 7615 (no ratchet file: add one first), second-format and refused unchanged | medium |
| Java | 1,462 | `decl.rs` parameters and lists, `stmt.rs` | `space.rs`, the compilation unit, comments | -250 / +100 | gjf real-world 4403, second-format 0; Java's own parser replacement (P6) touches the same files: order them | medium (+6.4% seen) |
| CSS/SCSS/Less | 2,259 | rule blocks and declaration lists (`block`, `key_value`), value comma lists (`delimited_list` with `Fill`) | `value.rs` math, URLs, SCSS maps, selector and at-rule preludes, `text.rs` after `literals` | -350 / +100 | prettier-css 108, scss 78, less 34, real-world 494 | low-medium |
| YAML | 1,451 | scalar quoting (`literals`), comment placement (shared), flow collections already on `delimited_list` | block collections, scalars folding, documents | -150 | yamllint and prettier-yaml ratchets | low |
| Markdown | 1,254 | embedded-code contract shared with HTML | everything else (line-oriented, prose as written) | -100 | markdown conformance | low |
| HTML + dialects | 3,936 | one embedded-code interface (script, style, Markdown fences, Vue/Svelte expressions) from `embed.rs` 685, Angular expressions on JS's roles | inline/block whitespace layout, Angular microsyntax, `model.rs` | -250 | prettier-html 129, vue 56, svelte 51, real-world 216/8/17, angular 56/681/444/11 | medium (P2 parser replacement touches `model.rs`: order them) |
| SQL | 1,827 | nothing: a token-stream layout by sqlfluff's rules, a family of its own | all | 0 | | none |
| Shell | 1,271 | dangling comments (done) | all (statement layout per shfmt) | 0-100 | shfmt | none |

## 6. Steps
Rule for every step: a mechanism needed by a second language (a layout construct, a check, an analysis) is built in the shared core in that step, never copied; that a language is older than the kit is never a reason it stays off it.

One step is one commit and one finish run, in one area. Each step records `scripts/cost.sh --languages` before and after, updates the docs of its area, and keeps every ratchet. Order: what unblocks the others first, then lines per effort. Lint (L) and format (F) tracks are independent and can run in parallel.

| # | Step | Removes (lang) / adds (shared) | Risk | After |
|---|---|---|---|---|
| L1 | `cargo xtask rename-rule` and multi-id catalog entries; rename the ids of section 3A that no code merge waits on | 0 / +270 | low | |
| F1 | `core_format::literals` (strings, numbers, comment text) on JS, Python, CSS, YAML | -250 / +120 | low | |
| L2 | Scope family complete (undefined, use-before-define, redeclared, shadowed, unused parameter, positions) with JS on it; ESLint ids are renamed; coverage by (rule, file, name) for eslint and typescript-eslint stays 100% with the mapping preset | -900 / +450 | medium-high | L1 |
| L3 | Python on the scope family (F4, F8, E74 rows); `binding.rs` goes | -650 / +150 | medium: ruff coverage 95.6% and 0 mismatches | L2 |
| F2 | List layouts and `shapes::signature`; JS objects, arrays and parameters, C# lists, Java parameters, Kotlin signature (opens the ktlint `function-signature` gap) | -900 / +500 | medium-high | F1 |
| F3 | `shapes::compound` with brace and offside bodies, and `shapes::maybe_parenthesize`; JS, C#, Java, Kotlin, PHP statements, YAML block collections, JS `return`/`throw`/arrow bodies and C# `paren_group` on them | -500 / +300 | medium | F2 |
| L4 | Roles `#expression-statement`, `#empty-statement`, `#try` for Ruby, the catch type field; shared rules `no-empty-catch`, `no-broad-catch`, `no-throw-generic`, `no-useless-expression`, `no-self-assign`, `boolean-literal-comparison`, `no-wildcard-imports`, banned-call rows (`no-print`, `no-eval`, `no-debugger`); CSS shared-id copies; Python jump placement as role rows | -400 / +120 | low-medium | L1, R4 |
| L5 | Flow family: `unreachable-code`, `no-fallthrough` on `core_flow::build` for every language with statement roles | -170 / +0 | low | L1 |
| F4 | Python formatter on the kit (`maybe_parenthesize` and offside `compound` from F3, lists, `binary_chain`, `conditional`, `call_chain`); a construct Python still needs that a second language has goes to the kit in this step | -1,300 / +400 | high | F2, F3 |
| L6 | `names`, `naming-convention`, `unique-names` rows, `max`: CSS families to `core-rules`, Kotlin and Protobuf rows, GraphQL naming, JS/Docker/Makefile/SQL/Markdown uniqueness | -650 / +450 | medium | L1 |
| F5 | JS second pass: call arguments hug builder, arrow chains, `needs_parens` as data, TS types lists | -600 / +250 | high | F2, F3 |
| F6 | CSS onto blocks, key-value and list layouts | -350 / +100 | medium | F2 |
| F7 | C# lists and braces, Java declarations and statements (after P6) | -750 / +300 | medium | F2, F3 |
| F8 | HTML and Markdown embedded-code interface, YAML leftovers | -300 | medium | F1 |

Estimated totals: lint -2,800 / +1,300, formatter -4,500 / +2,000. By lines per effort the first targets are L2, F2, F4 and L3; L1 and F1 come first because everything after uses them. Not planned: SQL and shell formatters (own families), doc-comment, regex-source, import-order and member-order families (triggers in 3B), any rename without a code merge behind it.

## 7. Decisions needed
1. Whether the formatter steps may spend decision 1's 2-point ratchet allowance (default: no).
2. `no-unused-vars` and ESLint's per-rule options becoming three ids (`unused-variable`, `unused-parameter`, `unused-import`), the old id gone; the alternative is one id `unused-variable` with a `report` option and no `unused-parameter`.
3. `no-empty-block` stops reporting catch bodies (they move to `no-empty-catch`): the same findings under another id for JS users.
