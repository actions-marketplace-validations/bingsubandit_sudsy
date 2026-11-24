# Go (`lang-go`)

## Parser
tree-sitter-go 0.25.0, forked in `grammars/tree-sitter-go` (FORK.md there): the upstream grammar rejects `new(expr)` (Go 1.26: `new(int32(1))`, `new("a")`, 236 files of the Kubernetes corpus) and a method with type parameters (`func (r *Rand) N[Int intType](n Int) Int`, the Go 1.27 standard library). The parser is chosen by the user (decision of the Go step); the corpus below parses without error in all its files.

`grammars/go.grammar` has a row per tree-sitter node (Pascal case) and groups the anonymous tokens: `Keyword`, `Op` (operators), `Assign`, `Define`, `Punct`, `Open` and `Close`. A leaf node that is a statement (`fallthrough`, a lone `;`) is a token wrapped in a statement node of its own (`Mapping::wrap`).

Files: `*.go`.

## Formatting
Not offered: gofmt ships with Go, so the file is kept as written (`core_format::VerbatimFormatter`).

## Lint
Revive's default rule set (catalog `docs/ai/parity/revive.yaml`, preset `go/revive`), measured by `scripts/parity.sh coverage revive` on the corpus below: 47,460 findings of 22 rules in 21,480 files, 32.0% of them reported on the same line by `unused-parameter` and `redefines-builtin-id` (the other rules wait).

`scope.rows` gives the shared scope family the names of Go: every `Ident` is a read unless a row declares it (a parameter, a `var` or `const` name, the names left of `:=`, a `range` clause, a `case v := <-ch`), and an assignment counts as a use (revive reads it so). `ScopeLang::late_declares` hides the name of a `:=`, `var` or `const` inside a function from reads written before its statement ends (`ctx := ctx.With()` and `if err != nil` before `x, err := f()` read the parameter). `unused-parameter` reports a parameter of a function with a body or of a function literal that nothing reads (`^_` names allowed). `redefines-builtin-id` is the shared `redefines-builtin` check (`scope_rules/builtins.rs`) on the `globals` words of `scope.rows`: a declaration, or a name left of `=` (`assigned_by: [AssignmentStatement]`), named like a predeclared identifier; a receiver (`except_parents`) and a method are left out. Python uses it for ruff A001/A002 with its builtin names.

Extra findings (same line, revive reports nothing; `only Sudsy on the line` of the run): `unused-parameter` 6,435 of 20,607 (31.2%) and `redefines-builtin-id` 1,035 of 2,039 (50.8%). All but 2 and 9 of them have one of these causes: files with the generated header (4,865 and 192; revive skips them: none of the 4,857 files with findings in the Kubernetes run has the header, 3,851 files have it; the shared lint option `skip_generated`, phase 5), files that the revive run does not load because the file name or a `//go:build` line excludes them for linux/arm64 (956 + 561 and 116 + 66; an environment fact, Sudsy lints every file it is given), folders that start with `_` (`_asm`, `_gen`; 50 and 21) and, for `redefines-builtin-id`, `min`, `max` and `clear` (631; revive 1.17.1 does not know these Go 1.21 builtins, Sudsy does). The rest: 2 `unused-parameter` findings where tree-sitter-go reads an index chain `a[i][j]` as a generic instantiation (`docs/ai/bugs.md`), and 9 `redefines-builtin-id` findings of a name in a `const` or `var` group (`true = 0 == 0` in `builtin.go`): revive reports a spec of a group at the line of the group's keyword and Sudsy at the name, so each is one extra and one missed finding (the 38 missed of 1,042).

A function body is no scope of its own (`Block` is a `#block`, not a `#scope`): in Go the parameters and the top-level names of the body are one scope, so `ml, err := f()` at the top of a body assigns the parameter `ml`. The tokens `true`, `false`, `nil` and `iota` are names (tree-sitter-go makes them tokens of their own, so a parameter named `iota` is read as `iota`).

Revive's type-dependent rules (`var-declaration`, `time-naming`, `unexported-return`, `context-keys-type`) give different findings on identical runs (three runs on the standard library: 18,641 to 18,647 findings, only these rules differ; three on Kubernetes: 25,399 to 25,402; `GOMAXPROCS=1` keeps it; the `missed` of the full run varies from 234 to 254 for `var-declaration`). Sudsy has no rule for them (they need go/types: gaps), so the `missed` of these four in `tests/parity/revive/differences.yaml` is the highest of several runs and a run does not fail on the jitter.

Revive skips a file whose header comment is `// Code generated ... DO NOT EDIT.`; Sudsy lints it like the rest, because `skip_generated` is a formatting option only and no shared option skips files in lint (a gap: Sudsy reports findings in generated files that revive does not). The comments `//revive:disable` and `//nolint` are not read: a `suppress directive` row only checks rule groups, it does not silence rules (open, same as Ruby's `rubocop:`).

### Mapping table
| revive rule (findings) | Entry | Status |
|---|---|---|
| exported (15,691), package-comments (2,355) | `missing-doc-comment` (Go form `// Name ...`), the package comment is a fact of the directory | index, rows; directory fact new |
| var-naming (10,923), error-naming (79) | `naming-convention` with the initialisms list as an option | index, rows |
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
<!-- /generated -->
