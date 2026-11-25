# Go (`lang-go`)

## Parser
tree-sitter-go 0.25.0, forked in `grammars/tree-sitter-go` (FORK.md there): the upstream grammar rejects `new(expr)` (Go 1.26: `new(int32(1))`, `new("a")`, 236 files of the Kubernetes corpus) and a method with type parameters (`func (r *Rand) N[Int intType](n Int) Int`, the Go 1.27 standard library). The parser is chosen by the user (decision of the Go step); the corpus below parses without error in all its files.

`grammars/go.grammar` has a row per tree-sitter node (Pascal case) and groups the anonymous tokens: `Keyword`, `Op` (operators), `Assign`, `Define`, `Punct`, `Open` and `Close`. A leaf node that is a statement (`fallthrough`, a lone `;`) is a token wrapped in a statement node of its own (`Mapping::wrap`).

Files: `*.go`.

## Formatting
Not offered: gofmt ships with Go, so the file is kept as written (`core_format::VerbatimFormatter`).

## Lint
Revive's default rule set (catalog `docs/ai/parity/revive.yaml`, preset `go/revive`), measured by `scripts/parity.sh coverage revive` on the corpus below: 47,476 findings of 23 rules in 21,480 files, 92.4% of them reported on the same line by `missing-doc-comment`, `doc-first-word`, `package-comment-form`, `missing-package-comment`, `unused-parameter`, `redefines-builtin-id` and `var-naming` (the other rules wait).

`scope.rows` gives the shared scope family the names of Go: every `Ident` is a read unless a row declares it (a parameter, a `var` or `const` name, the names left of `:=`, a `range` clause, a `case v := <-ch`), and an assignment counts as a use (revive reads it so). `ScopeLang::late_declares` hides the name of a `:=`, `var` or `const` inside a function from reads written before its statement ends (`ctx := ctx.With()` and `if err != nil` before `x, err := f()` read the parameter). `unused-parameter` reports a parameter of a function with a body or of a function literal that nothing reads (`^_` names allowed). `redefines-builtin-id` is the shared `redefines-builtin` check (`scope_rules/builtins.rs`) on the `globals` words of `scope.rows`: a declaration, or a name left of `=` (`assigned_by: [AssignmentStatement]`), named like a predeclared identifier; a receiver (`except_parents`) and a method are left out. Python uses it for ruff A001/A002 with its builtin names.

Doc comments (revive `exported`, `package-comments`; 15,409 of 15,691 and 2,303 of 2,355 findings, `lang-go/rules/doc-comments.yaml`): a doc is the run of `//` (or `/* */`) comments right above a declaration, a blank line ends it (`doc-lines`, `doc-run`); lines of directives (`//go:`, `line `, `extern `, `export `) are no text (`doc-skip`) and a first line `Deprecated: ` counts as no doc. `missing-doc-comment` reports an exported function, method, type, constant or variable (`visible-by-case`: first rune uppercase) without one; a method needs an exported receiver type (`receiver`, `receiver-token`), `Error`, `Read`, `ServeHTTP`, `String`, `Write`, `Unwrap` and `Len`/`Less`/`Swap` of a type that has all three are skipped; a constant or variable in a group with a group comment is excused (`group-comment`) and only the first of a group without one is reported (`once`). `doc-first-word` (check `first-word` of the `doc-comment` family) wants the name as first word, a type may start with `A`, `An`, `The` or `This`; a spec in a group whose comment has text is not form-checked. Files of package `main` are skipped (`skip-namespaces`) and so, by the preset's `overrides`, are `_test.go` files.

The package comment is a fact of the directory (`unit-doc-comment` family, rule `missing-package-comment`): one report at the `package` keyword of the first file by name when some file that is not a test file has no comment and no file of the directory has one (a test file's comment counts; revive keys its map by path, so the first file by name takes the report). A file with a detached comment (right form, blank line before `package`) is reported by `package-comment-form` and never causes the report. The unit is the directory read from disk, also when only some of its files are linted; with a file that does not parse the rule is silent.

Gaps: stuttering (`exported`: a name that starts with the package name; it needs the package name and does not fit an existing names check), the own-declaration check for `var a, B`, a method that is sortable through methods in another file, 282 other missed `exported` findings (not classified: `missed` lists them) and 52 missed `package-comments` findings where a generated or build-excluded sibling that revive does not load holds the first file name or the only comment.

Extra findings (same line, revive reports nothing; `only Sudsy on the line` of the run): `var-naming` 8,292 of 19,204 (43.2%), `missing-doc-comment` 2,509 of 17,785 (14.1%), `unused-parameter` 1,584 of 15,756 (10.1%), `redefines-builtin-id` 874 of 1,882 (46.4%) and `missing-package-comment` 81 of 2,425 (3.3%). Files with the generated header are no cause any more (`skip_generated_lint`, below). The causes are files that the revive run does not load because the file name or a `//go:build` line excludes them for linux/arm64 (7,702, 2,449, 1,495, 180 and 40; an environment fact, Sudsy lints every file it is given) and folders that start with `_` (`_asm`, `_gen`; 488, 56, 50, 21 and 23), and for `redefines-builtin-id` `min`, `max` and `clear` (revive 1.17.1 does not know these Go 1.21 builtins, Sudsy does; most of the 672 others). Not classified: 102 `var-naming` (build lines the classifier does not read), 25 `unused-parameter` (a real unused parameter that revive reports nowhere, `stateError(s *scanner, c byte)` in `encoding/json/scanner.go`) and 15 `missing-package-comment` at line 0 (revive reports the package comment at line 0 or 1 by file). 9 `redefines-builtin-id` findings of a name in a `const` or `var` group (`true = 0 == 0` in `builtin.go`): revive reports a spec of a group at the line of the group's keyword and Sudsy at the name, so each is one extra and one missed finding.

`var-naming` is the shared `naming-convention` family (`rules/var-naming.yaml`) with the convention `words` (`core_rules::word_case`, a port of revive's `Name`: no `_` inside a name, no ALL_CAPS word, initialisms in capitals from the option `initialisms`, revive's list by default) and the option `ignore_pattern` (`^(LastInsertId|kWh)$`; the preset adds `Example`, `Test`, `Benchmark` and `Fuzz` names for `_test.go`). A path ending in `:each` names every part (`a, b int`). It covers the names of functions, parameters, results, interface methods, constants, variables, types, fields and the names left of `:=`; receivers are left to `receiver-naming`.

A function body is no scope of its own (`Block` is a `#block`, not a `#scope`): in Go the parameters and the top-level names of the body are one scope, so `ml, err := f()` at the top of a body assigns the parameter `ml`. The tokens `true`, `false`, `nil` and `iota` are names (tree-sitter-go makes them tokens of their own, so a parameter named `iota` is read as `iota`).

Revive's type-dependent rules (`var-declaration`, `time-naming`, `unexported-return`, `context-keys-type`) give different findings on identical runs (three runs on the standard library: 18,641 to 18,647 findings, only these rules differ; three on Kubernetes: 25,399 to 25,402; `GOMAXPROCS=1` keeps it; the `missed` of the full run varies from 234 to 254 for `var-declaration`). Sudsy has no rule for them (they need go/types: gaps), so the `missed` of these four in `tests/parity/revive/differences.yaml` is the highest of several runs and a run does not fail on the jitter.

Revive skips a file whose header comment is `// Code generated ... DO NOT EDIT.`: `lang.yaml` sets the shared options `skip_generated_lint` (no finding of any rule, and the file is no cause or answer of a directory fact such as the package comment; also in the editor) and `generated_header_markers` (`code generated *do not edit.`, `*` is any text within a line). The comments `//revive:disable` and `//nolint` are not read: a `suppress directive` row only checks rule groups, it does not silence rules (open, same as Ruby's `rubocop:`).

### Mapping table
| revive rule (findings) | Entry | Status |
|---|---|---|
| exported (15,691), package-comments (2,355) | `missing-doc-comment`, `doc-first-word`, `package-comment-form`, `missing-package-comment` (directory fact) | done (covers 15,409 and 2,303); stuttering gap |
| var-naming (10,923) | `naming-convention` with the `words` convention and the option `initialisms` | done (covers 10,912) |
| error-naming (79) | pattern rule | index |
| unused-parameter (14,178) | `scope-rules` rows (`scope.rows`) | done (covers 14,172) |
| redefines-builtin-id (1,042) | shared `redefines-builtin` check, the predeclared names as `globals` | done (covers 1,004) |
| receiver-naming (899) | one name per receiver type in the file (revive keeps the map per file) | new, shared |
| indent-error-flow (472), superfluous-else (75) | `simplify` rows | index, rows |
| empty-block (320), unreachable-code (22) | `no-empty-block`, flow family (`goto`, labeled `break`, `panic`) | index |
| dot-imports (297), blank-imports (82), increment-decrement (189), error-strings (183), context-as-argument (237), error-return (18), range (7) | pattern rules (YAML) or a row of an existing family | index |
| var-declaration (238), unexported-return (138), context-keys-type (11), errorf (3), time-naming | `gap`: needs go/types (revive type-checks the package) | gap |

## Conformance
`tests/conformance/fetch-go-corpus.sh` fetches the `.go` files of Kubernetes (`eb180eec`), Hugo (`7f0b4416`), Prometheus (`ad8f26ee`) and the Go 1.27.4 standard library at pinned commits, without `vendor` and `testdata` folders.

## Options
<!-- generated:options -->
| Option | Values | Default | Description |
|---|---|---|---|
| `indent_style` | `space`, `tab` | `space` | Indent with spaces or tabs. |
| `final_newline` | bool | `true` | End the file with a newline. |
| `end_of_line` | `auto`, `lf`, `crlf` | `auto` | Line break style of the output: `lf`, `crlf`, or `auto` to keep the file's first line break. |
| `skip_generated_lint` | bool | `true` | Report nothing in generated files: a file whose name matches `generated_names` or whose leading comments contain one of `generated_header_markers`. |
| `generated_header_markers` | list of words | `[code generated *do not edit.]` | Texts (`<auto-generated`) that mark a generated file when they occur, case-insensitively, in the comments at the start of the file; a `*` stands for any text within one line (`code generated *do not edit.`). Read when `skip_generated` or `skip_generated_lint` is on. |
<!-- /generated -->
