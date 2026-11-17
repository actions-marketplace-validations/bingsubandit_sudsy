# Ruby (`lang-ruby`)

## Parser
Prism (the parser of CRuby 3.4+; Ruby 3.3 to 4.0 syntax; `ruby-prism-sys` pinned `=2.1.0`, the library's C sources built by `cc`, bumped by hand) behind `crates/lang-ruby/src/adapter.rs`, for parsing only: layout and rules are ours. The Rust crate `ruby-prism` is not a dependency: it wraps neither the lexer's token callback nor a way to build its result from our own parser call. `adapter/ffi.rs` (the crate's only `unsafe`, allowed by a `deny` in its manifest) makes one parse with the C library's lex callback set and then visits the tree with the library's visitor: tokens and node spans of the same parse, no second pass. The adapter drops newlines, comments and the separators of word lists, which `provider_api::adapter::replay` rebuilds from the text between tokens (`#` comments, `=begin` blocks, a `\` that continues a line); the text after `__END__` is one `Data` token that the formatter keeps byte for byte (`NodeFormatter::exact_end`).

Why Prism, measured against tree-sitter-ruby 0.23.1 and lib-ruby-parser 4.0.6 on the same 18,958 files (86.5 MB; aarch64 Graviton, release, parse only): parse errors 0, 5 and 15; speed 124-178, 7-11 and 19-28 MB/s; Prism is the reference for what CRuby accepts (it rejects `a[&]`, which 3.4 forbids; tree-sitter accepts it) and follows each Ruby release (ruby/prism, MIT, CRuby core team); lib-ruby-parser stops at Ruby 3.1.

`grammars/ruby.grammar` has a row per Prism node type (`CallNode` is `Call`, `SelfNode` is `This`; the implicit block parameters `it` and `_1`, which Prism spans over the whole block, have none) and opts into `roles`: 67 kinds have a role, 83 are `#opaque` (table in [../architecture/syntax-tree.md](../architecture/syntax-tree.md)). Prism makes operators and index reads method calls, so `a + b` and `a[1]` are `Call`s (`#call` and `#member-access`) and only `and`, `or` and ranges are `#binary`; `if`, `unless` and the ternary are `If`/`Unless` and `#opaque`. Two places where Prism's spans do not follow the text, which the adapter corrects:
- A method or block body with `rescue` is a `Begin` that Prism spans over the whole method or block; the adapter makes it span its clauses.
- A heredoc's body comes after the rest of its opening line, and Prism ends the nodes of that line before it. The body (text, interpolations, closing label) is one `HeredocBody` token with no nodes of its own, and the nodes that hold the whole line are extended over it, so it is a child of the innermost of them.

Files: `*.rb`, `*.rake`, `*.gemspec`, `*.ru`, `Gemfile`, `Rakefile`, `Guardfile`, `Capfile`.

## Formatting
Not formatted yet: the file is kept as written.

## Conformance
`tests/conformance/fetch-ruby-corpus.sh` fetches Rails, Discourse, RuboCop and Jekyll at pinned commits (`.rb` and `.rake` files, 18,958 files): every file parses without error, prints back byte for byte, its tree fits the grammar (`ShapeChecker`) and it formats with the same tokens and idempotently (`cargo test --profile corpus -p lang-ruby --test integration -- --ignored --nocapture conformance`, 34 s). Parse speed per repository: `cargo test --release -p lang-ruby --test integration -- --ignored --nocapture speed` ([../performance/numbers.md](../performance/numbers.md)).

## Options
<!-- generated:options -->
| Option | Values | Default | Description |
|---|---|---|---|
| `indent_size` | int 1-16 | `2` | Columns per indentation level (also the width of a tab). |
| `line_width` | int 20-1000 | `120` | Preferred maximum line width in columns. |
| `indent_style` | `space`, `tab` | `space` | Indent with spaces or tabs. |
| `final_newline` | bool | `true` | End the file with a newline. |
| `end_of_line` | `auto`, `lf`, `crlf` | `auto` | Line break style of the output: `lf`, `crlf`, or `auto` to keep the file's first line break. |
<!-- /generated -->
