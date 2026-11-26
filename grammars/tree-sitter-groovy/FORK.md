# Fork of tree-sitter-groovy

Upstream: crates.io `dekobon-tree-sitter-groovy` 0.6.0 (dekobon/tree-sitter-groovy, Apache-2.0 OR MIT, [LICENSE-APACHE](LICENSE-APACHE), [LICENSE-MIT](LICENSE-MIT)). Regenerate `src/` after editing `grammar.js` with `scripts/grammar-regen.sh groovy`.

`bindings/rust` is trimmed to the language constant (no doctest, no stress test); `tree-sitter.json` is added for the CLI.

Changes to `grammar.js` (command expressions of Gradle scripts that the upstream grammar rejected):
- `command_chain`: the first argument of a parenthesis-free call may be any expression when a comma follows it (`dependsOn a, b`, `proguardFiles f, 'x'`); without a comma `foo bar` stays two statements, because a newline is not a token. Conflicts with `_expression`, `_type` and `_command_argument` are resolved by the GLR parser.
- `command_chain`: the receiver may be a dotted name (`inputs.property "k", v`) when the first argument is an expression with a comma or a named argument (`foo.bar baz, qux: 1`). A dotted receiver with one plain argument stays two statements.
- A string may be the key of a named argument (`attributes 'k': v`, `expand "k": v`).
- `variable_declarator`: the value may be a command with several arguments (`def t = project.tasks.create "x", Jar`).

- `lambda_expression`: a Java-style lambda (`x -> x.y`, `(Configuration it) -> { ... }`), which Groovy 3 accepts; it conflicts with the first parameter of a closure, where the closure wins.

Corpus (1,538 `.gradle` files of 43 projects, `docs/ai/design/gradle.md`): files without an error node 1,374 (89.3%) before, 1,516 (98.6%) after; of all 6,481 files 6,442 (99.4%) parse; probes failing 4 of 70 before, 1 after (`[*a, *: m]`). Speed 18.7 MB/s before, 12.7 MB/s after (more GLR conflicts). Not fixed: a spread in a list literal, a string-named method call over several lines, the 39 files that are templates (`<% %>`, `@x@`) or rare forms.
