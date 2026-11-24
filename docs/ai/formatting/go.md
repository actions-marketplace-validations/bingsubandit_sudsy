# Go (`lang-go`)

## Parser
tree-sitter-go 0.25.0, forked in `grammars/tree-sitter-go` (FORK.md there): the upstream grammar rejects `new(expr)` (Go 1.26: `new(int32(1))`, `new("a")`, 236 files of the Kubernetes corpus) and a method with type parameters (`func (r *Rand) N[Int intType](n Int) Int`, the Go 1.27 standard library). The parser is chosen by the user (decision of the Go step); the corpus below parses without error in all its files.

`grammars/go.grammar` has a row per tree-sitter node (Pascal case) and groups the anonymous tokens: `Keyword`, `Op` (operators), `Assign`, `Define`, `Punct`, `Open` and `Close`. A leaf node that is a statement (`fallthrough`, a lone `;`) is a token wrapped in a statement node of its own (`Mapping::wrap`).

Files: `*.go`.

## Formatting
Not offered: gofmt ships with Go, so the file is kept as written (`core_format::VerbatimFormatter`).

## Lint
Revive's default rule set (catalog `docs/ai/parity/revive.yaml`, preset `go/revive`), measured by `scripts/parity.sh coverage revive` on the corpus below: 47,459 findings of 22 rules in 21,480 files. Revive reads a package at a time: a rule that needs the other files of the directory (`package-comments`; `exported` for types that implement `sort.Interface`) needs a directory fact, not a second pass over a file.

Revive skips a file whose header comment is `// Code generated ... DO NOT EDIT.`; Sudsy lints it like the rest, because `skip_generated` is a formatting option only and no shared option skips files in lint (a gap: Sudsy reports findings in generated files that revive does not). The comments `//revive:disable` and `//nolint` are not read: a `suppress directive` row only checks rule groups, it does not silence rules (open, same as Ruby's `rubocop:`).

### Mapping table
| revive rule (findings) | Entry | Status |
|---|---|---|
| exported (15,691), package-comments (2,355) | `missing-doc-comment` (Go form `// Name ...`), the package comment is a fact of the directory | index, rows; directory fact new |
| var-naming (10,923), error-naming (79) | `naming-convention` with the initialisms list as an option | index, rows |
| unused-parameter (14,178) | `scope-rules` rows (`scope.rows`) | index, rows |
| redefines-builtin-id (1,042) | `shadowed-variable` with a builtin-name table | index, data |
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
