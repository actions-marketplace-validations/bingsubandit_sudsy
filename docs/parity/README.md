# Parity catalog

What the tools Sudsy aims to replace do, and where Sudsy stands. One file per tool (`docs/parity/<tool>.yaml`); `tool`, `version` (upstream; the parity runner installs it), `languages`, `sources` and `scope` head each file. A test (`crates/languages/tests/parity.rs`) fails when a `rule` or `option` status names something that does not exist; the counts below are generated (`cargo xtask codegen`). The differential runner [`scripts/parity.sh`](../development/parity.md) compares a tool's output with `sudsy lint` on fixtures.

## Format
The catalog of a lint tool lists every rule of the pinned version (`scripts/parity_inventory.py <tool> --write` appends the missing ones as `planned`; a deprecated rule as `wontfix`, a ruff preview rule as `deferred`), so the counts below are inventory sizes, not progress: how much of a tool's output Sudsy reproduces is the Coverage share column (`scripts/parity.sh coverage <tool>`, [../development/parity.md](../development/parity.md#coverage-mode); `-` where the tool has no corpus yet). One line per upstream rule or option under `entries:`, `<id>: <status>` or `<id>: <status> | <gap>`; the id of an option is `line-length.max`, so tools with many sub-options weigh more in the counts. `<gap>` says what is missing and makes a `rule`, `option`, `formatter` or `config` entry `partial`; for `planned` and `deferred` it is the reason. Ids that are only deferred are grouped by reason under a top-level `deferred:` (`- gap: "<reason>"`, `ids: [DL3049, DL3050]`). `languages::catalog::parse` expands both forms; coverage follows from the status, so it is not written.

| Status | Meaning | Coverage |
|---|---|---|
| `rule <id>` | A Sudsy rule does it (`syntax-error` is the parser's diagnostic); the rule's `sudsy rules` page and the table in [rules/builtin.md](../rules/builtin.md) list the entry as its equivalent. | covered, partial with a gap |
| `option <language>.format.<name>` | A formatter option does it. | covered, partial with a gap |
| `formatter` | Fixed formatter behavior, no option. | covered, partial with a gap |
| `config <text>` | Configuration outside rules and options (overrides, ignore files, presets, suppression comments). | covered, partial with a gap |
| `sdk <id>` | A .NET SDK diagnostic (IDE, CA, CS) does it; Sudsy never reimplements these. | sdk |
| `planned` | Accepted for implementation. | missing |
| `deferred` | Wanted eventually, deliberately not scheduled. | deferred |
| `wontfix <reason>` | Out of scope. | wontfix |

## Counts
<!-- generated:counts -->
| Tool | Coverage share | Entries | Covered | Partial | Missing | SDK | Wontfix | Deferred | Planned |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| actionlint | 5.4% | 26 | 5 | 4 | 0 | 0 | 0 | 17 | 0 |
| angular-eslint | - | 37 | 3 | 2 | 22 | 0 | 0 | 10 | 22 |
| biome | - | 34 | 13 | 6 | 2 | 0 | 13 | 0 | 2 |
| buf | 0.1% | 47 | 20 | 3 | 0 | 0 | 0 | 24 | 0 |
| checkmake | 0.0% | 5 | 4 | 1 | 0 | 0 | 0 | 0 | 0 |
| checkstyle | 0.5% | 126 | 36 | 12 | 72 | 0 | 6 | 0 | 72 |
| csharpier | - | 16 | 11 | 5 | 0 | 0 | 0 | 0 | 0 |
| dotnet-format-editorconfig | - | 56 | 50 | 3 | 0 | 1 | 2 | 0 | 0 |
| eslint | 0.9% | 292 | 93 | 21 | 125 | 0 | 32 | 21 | 125 |
| eslint-plugin-jsonc | - | 44 | 36 | 6 | 0 | 0 | 2 | 0 | 0 |
| eslint-plugin-svelte | 100.0% | 85 | 19 | 9 | 54 | 0 | 3 | 0 | 54 |
| eslint-plugin-vue | 87.2% | 256 | 45 | 58 | 149 | 0 | 4 | 0 | 149 |
| google-java-format | - | 14 | 2 | 5 | 1 | 0 | 6 | 0 | 1 |
| graphql-eslint | - | 65 | 4 | 6 | 30 | 0 | 0 | 25 | 30 |
| hadolint | 3.4% | 71 | 38 | 10 | 0 | 0 | 1 | 22 | 0 |
| html-eslint | 0.2% | 66 | 13 | 12 | 41 | 0 | 0 | 0 | 41 |
| html-validate | 10.4% | 97 | 34 | 30 | 26 | 0 | 1 | 6 | 26 |
| htmlhint | 0.3% | 44 | 10 | 6 | 28 | 0 | 0 | 0 | 28 |
| ktlint | 12.0% | 97 | 34 | 7 | 56 | 0 | 0 | 0 | 56 |
| markdownlint | 0.0% | 53 | 25 | 16 | 0 | 0 | 0 | 12 | 0 |
| phpstan | 0.0% | 1 | 0 | 1 | 0 | 0 | 0 | 0 | 0 |
| prettier | - | 59 | 38 | 9 | 2 | 0 | 9 | 1 | 2 |
| psscriptanalyzer | 3.6% | 75 | 12 | 8 | 0 | 0 | 1 | 54 | 0 |
| roslynator | - | 340 | 23 | 7 | 289 | 18 | 1 | 2 | 289 |
| rubocop | 96.2% | 160 | 27 | 5 | 69 | 0 | 0 | 59 | 69 |
| ruff | 95.6% | 980 | 133 | 0 | 689 | 0 | 19 | 139 | 689 |
| shellcheck | 31.0% | 520 | 17 | 8 | 0 | 0 | 0 | 495 | 0 |
| shfmt | - | 9 | 1 | 1 | 0 | 0 | 3 | 4 | 0 |
| sqlfluff | 0.1% | 97 | 35 | 0 | 43 | 0 | 4 | 15 | 43 |
| stylecop | - | 197 | 120 | 10 | 1 | 45 | 21 | 0 | 1 |
| stylelint | 0.1% | 150 | 83 | 9 | 54 | 0 | 0 | 4 | 54 |
| stylelint-less | - | 6 | 0 | 0 | 5 | 0 | 0 | 1 | 5 |
| stylelint-scss | - | 71 | 1 | 2 | 67 | 0 | 0 | 1 | 67 |
| taplo | - | 23 | 11 | 7 | 0 | 0 | 0 | 5 | 0 |
| terraform | - | 2 | 1 | 1 | 0 | 0 | 0 | 0 | 0 |
| tflint | 0.0% | 20 | 8 | 4 | 0 | 0 | 0 | 8 | 0 |
| typescript-eslint | 0.0% | 136 | 10 | 16 | 34 | 0 | 9 | 67 | 34 |
| yamllint | 0.2% | 66 | 62 | 3 | 1 | 0 | 0 | 0 | 1 |
<!-- /generated -->

## Lint coverage by function
Default is Sudsy with no `sudsy.yaml` (the ratchet); all mapped rules on is the same run with every mapped rule enabled. The share of the findings of each reference tool's default or recommended set on its real-world corpus that Sudsy also reports in the same file (`scripts/parity.sh coverage <tool>`, [../development/parity.md](../development/parity.md#coverage-mode); the ratchet is `coverage:` in `tests/parity/<tool>/differences.yaml`). Missing is the number of findings Sudsy does not report; the five rules with most of them follow, with their counts (`ruff` is measured by its own step: its top gaps are in the parity doc).

| Language | Tool | Default | All mapped rules on | Mapped rules off by default (findings) | Top 5 missing rules (findings) |
|---|---|---:|---:|---|---|
| Shell | shellcheck | 31.0% | 47.0% | `SC2155` 33, `SC2006` 3 | `SC2140` 22, `SC2034` 14, `SC1091` 12, `SC2154` 11, `SC2206` 9 |
| Dockerfile | hadolint | 3.4% | 27.6% | `DL3008` 116, `DL3047` 86, `DL3018` 83, `DL3003` 70, `DL3066` 48, `SC2046` 30 | `DL4006` 153, `DL3003` 64, `DL3008` 54, `DL3047` 50, `DL3018` 42 |
| YAML | yamllint | 0.2% | 73.8% | `line-length` 1164, `document-start` 641, `truthy` 394 | `comments` 325, `indentation` 160, `syntax` 136, `braces` 57, `document-start` 52 |
| Markdown | markdownlint | 0.0% | 82.7% | `MD013` 1222, `MD033` 209, `MD025` 36, `MD040` 21, `MD024` 21, `MD036` 10 | `MD010` 309, `MD037` 6, `MD041` 4, `MD051` 2, `MD028` 1 |
| CSS | stylelint | 0.1% | 13.4% | `custom-property-pattern` 462, `no-descending-specificity` 413, `number-max-precision` 221, `selector-class-pattern` 218, `property-no-vendor-prefix` 141 | `rule-empty-line-before` 3832, `color-function-notation` 3181, `color-function-alias-notation` 1428, `alpha-value-notation` 1271, `media-feature-range-notation` 415 |
| HTML | html-validate | 10.4% | 63.7% | `no-trailing-whitespace` 162, `void-style` 129, `no-inline-style` 63, `no-implicit-button-type` 49, `presentation-role-style` 32, `doctype-style` 25 | `wcag/h63` 48, `element-permitted-content` 23, `wcag/h30` 23, `parser-error` 21, `element-required-content` 20 |
| HTML | htmlhint | 0.3% | 5.0% | `tag-pair` 107, `doctype-first` 87, `attr-value-double-quotes` 9 | `spec-char-escape` 1087, `tag-pair` 105, `doctype-first` 36, `src-not-empty` 6, `attr-value-double-quotes` 1 |
| HTML | html-eslint | 0.2% | 0.3% | `require-closing-tags` 155, `no-multiple-h1` 45, `no-duplicate-attrs` 37 | `indent` 16182, `element-newline` 2175, `attrs-newline` 1158, `require-closing-tags` 146, `no-extra-spacing-tags` 132 |
| SQL | sqlfluff | 0.1% | 4.4% | `AL01` 1199 | `CP02` 5770, `CP01` 3992, `LT05` 1368, `AM04` 1121, `AL03` 1061 |
| Protobuf | buf | 0.1% | 85.9% | `ENUM_VALUE_PREFIX` 1375, `PACKAGE_VERSION_SUFFIX` 231, `ENUM_ZERO_VALUE_SUFFIX` 150, `RPC_RESPONSE_STANDARD_NAME` 120, `FIELD_LOWER_SNAKE_CASE` 91 | `RPC_REQUEST_RESPONSE_UNIQUE` 117, `PACKAGE_DIRECTORY_MATCH` 98, `RPC_RESPONSE_STANDARD_NAME` 64, `FIELD_NOT_REQUIRED` 39, `RPC_REQUEST_STANDARD_NAME` 9 |
| HCL | tflint | 0.0% | 19.6% | `terraform_unused_declarations` 45, `terraform_required_providers` 37, `terraform_required_version` 34, `terraform_typed_variables` 27 | `terraform_required_providers` 37, `terraform_unused_declarations` 31, `terraform_typed_variables` 27, `terraform_required_version` 20 |
| GitHub Actions | actionlint | 5.4% | 5.4% | - | `syntax-check` 235, `expression/types` 155, `events` 115, `action` 41, `glob` 22 |
| Kotlin | ktlint | 12.0% | 13.2% | `max-line-length` 1143 | `function-signature` 30800, `argument-list-wrapping` 24113, `multiline-expression-wrapping` 15169, `class-signature` 4591, `trailing-comma-on-call-site` 4162 |
| Java | checkstyle | 0.5% | 25.5% | `LineLength` 13166, `MethodName` 7940, `LocalVariableName` 339 | `MissingJavadocMethod` 18283, `MethodName` 7887, `JavadocParagraph` 5301, `SummaryJavadoc` 1527, `TodoFormat` 1498 |
| PowerShell | psscriptanalyzer | 3.6% | 34.4% | `PSAvoidUsingWriteHost` 175, `PSUseApprovedVerbs` 116, `PSAvoidUsingEmptyCatchBlock` 21, `PSAvoidUsingCmdletAliases` 13 | `PSUseShouldProcessForStateChangingFunctions` 140, `PSUseDeclaredVarsMoreThanAssignments` 130, `PSUseCmdletCorrectly` 123, `PSReviewUnusedParameter` 123, `PSUseSingularNouns` 59 |
| PHP | phpstan | 0.0% | 0.0% | `variable.undefined` 591 (mapped, reports nothing) | `class.notFound` 7407, `variable.undefined` 591, `classConstant.notFound` 511, `staticMethod.notFound` 401, `function.notFound` 323 |
| Makefile | checkmake | 0.0% | 73.5% | `phonydeclared` 249, `uniquetargets` 155, `minphony` 108, `maxbodylength` 61 | `uniquetargets` 88, `phonydeclared` 59, `minphony` 5 |
| JavaScript | eslint | 0.9% | 100.0% | `no-undef` 4081, `no-unused-vars` 264, `no-useless-escape` 19, `no-case-declarations` 7 | - |
| TypeScript | typescript-eslint | 0.0% | 100.0% | `@typescript-eslint/no-explicit-any` 1643, `@typescript-eslint/no-unused-vars` 698, `no-var` 493, `@typescript-eslint/no-empty-object-type` 254, `@typescript-eslint/no-unused-expressions` 181 | - |
| Vue | eslint-plugin-vue | 87.2% | 99.6% | `vue/attributes-order` 81, `vue/html-self-closing` 71, `vue/no-v-html` 25 | - |
| Svelte | eslint-plugin-svelte | 100.0% | 100.0% | - | - |

Not measured:

| Tool | Reason |
|---|---|
| ruff, RuboCop | measured by their own steps |
| C# analyzers (StyleCop, Roslynator) | the compiler skips analyzers when a compilation has errors, and the C# corpus (the .NET runtime's CoreLib sources) does not compile on its own (118,604 compiler errors), so StyleCop reports nothing on it; needs a compiling corpus project or an analyzer driver that tolerates errors |
| clang-tidy | no catalog, and Sudsy has no C or C++ lint rules; a share would be 0 by construction |
| detekt, PMD, PHPCS | not catalogued |
| eslint-plugin-jsonc | the JSON corpus is a parser test suite (JSONTestSuite), no real-world files |
| stylelint-scss, stylelint-less | plugin rules without a recommended set of their own; need SCSS and Less configs and parsers |
| angular-eslint, graphql-eslint, biome, taplo, terraform | no coverage entry yet (fixtures only) |

Sets and corpora of each run: [../development/parity.md](../development/parity.md#coverage-mode). Findings of `formatter` entries are left out; a file the tool cannot read is left out and counted by the run. The layout rules that are `planned` in a catalog (html-eslint `indent`, ktlint `function-signature`, sqlfluff `CP01`) count as missing until the catalog says `formatter` ([../roadmap.md](../roadmap.md#small-tasks)).

## Migration
`sudsy init --from <tool>` ([config/cli.md](../config/cli.md#init---from)) turns a tool's config into a `sudsy.yaml` with this catalog as the source of truth: the catalog files are embedded in the binary, the reason printed for a setting that cannot be migrated is the entry's `gap` (or its status), and the C# `.editorconfig` entries that switch off covered SA rules come from the `covered` StyleCop entries. The option and rule values are mapped in code (`crates/cli/src/init/`); each mapping names its catalog entry and debug builds fail when the entry points elsewhere. A new `rule:` or `option:` status therefore changes what `init` writes for the settings that use it; the fixtures under `tests/fixtures/init/` show the result.

## Equivalent ids of rules
`sudsy rules <id>` and the table in [rules/builtin.md](../rules/builtin.md) list as `Equivalent` every `covered` or `partial` entry whose status is `rule <id>` in a catalog for one of the rule's languages (`languages::catalog::equivalents`). Rule descriptions do not name other tools; add or fix the entry here instead.
