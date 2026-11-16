# Design: Angular templates

Temporary: delete this file when Angular is done (formatter parity and lint rules landed).

## Choice
`*.component.html` is a dialect of the HTML tree, parsed by oxc-htmlish-parser (`Dialect::Angular`), expressions formatted by `lang-js`.

| Option | Structure | Expressions | Cost | Verdict |
|---|---|---|---|---|
| tree-sitter grammar for Angular | grammar to write and maintain (control flow, ICU, `@let`, microsyntax) | oxc on spans | a grammar of its own | rejected |
| our HTML grammar plus an external scanner | HTML parts reused; blocks and ICU need scanner states, `{`/`}` in text are ambiguous | oxc on spans | a scanner beside the grammar | rejected |
| oxc-htmlish-parser, `Dialect::Angular` | Angular's compiler reading in Rust: blocks, `@let`, ICU, case-sensitive names, template bindings | oxc on spans (`lang-js`) | one crate; `adapter::replay` builds our tree from its spans | chosen |

- The parser stops at the first error: a broken file is one syntax-error finding and no tree (accepted).
- Pipes are no TypeScript: a splitter in `format/print/angular/pipes.rs` separates them before oxc sees the expression.
- The crate is pinned exactly (`=0.0.5`) and ignored by Dependabot: its AST is not stable.
