# Fork of tree-sitter-go

Upstream: crates.io `tree-sitter-go` 0.25.0 (tree-sitter/tree-sitter-go, MIT, [LICENSE](LICENSE)). Regenerate `src/` after editing `grammar.js` with `scripts/grammar-regen.sh go`.

`bindings/rust` is trimmed to the language constant (no queries or doctest) so the workspace's fmt and clippy pass; `tree-sitter generate` does not touch it.

Changes to `grammar.js` (valid Go the upstream grammar rejects; tests in `crates/lang-go/tests/go.rs`):
- `special_argument_list`: the first argument of `new` and `make` may be an expression, not only a type (Go 1.26 `new(expr)`, `new(int64(300))`). `prec.dynamic(1, $._type)` keeps a type the preferred reading.
- `method_declaration`: optional `type_parameters` (a method with its own type parameters, as in `math/rand/v2`). A conflict `[type_parameter_declaration, _simple_type]` resolves the receiver ambiguity.

Error recovery: on corpus files with one bracket removed (5,210 mutants), the median share of tokens lost to ERROR nodes is the same as upstream, and the p90 is 9.25% against 8.91% upstream (+0.34 points). A separate alternative for `new` does not generate (unresolved conflict); a rule of its own for the argument list of `new` measures 9.21%.
