# Design: Ruby

Goal: formatter and linter for `.rb`, `.rake`, `.gemspec`, `.ru` files (Ruby 3.3 to 4.0 syntax). The parser approach is the user's decision (CLAUDE.md); this note measures the candidates and recommends one. Nothing is built yet.

## 1. Parser comparison (measured, not committed)
Linux arm64 Graviton, 8 cores, rustc 1.99.0, release with LTO, parse of the whole file from memory, warm, best of 5 rounds, no adapter. Corpus: 18,958 `.rb`/`.rake` files (86.5 MB, pinned tarballs): rails/rails `a5203b3`, discourse/discourse `47f9840`, rubocop/rubocop `dacb36c`, jekyll/jekyll `97c3bdc`. Probes: 11 hand-written files and 34 one-feature snippets (pattern matching incl. find pattern and pin expressions, endless methods, `it`, `_1`, hash shorthand, anonymous `*`/`**`/`&`/`...` forwarding, squiggly/nested/stacked heredocs, `%w %i %q %Q %r %s %x`, `?a`, rational/imaginary, `Data.define`, `BEGIN`/`END`, `__END__`, flip-flop, rescue modifier). Mutants: one `)`, `}`, `]` or `end` deleted at a pseudo-random place of 14,800 valid files (500 B to 40 KB). No `ruby` is installed, so "valid" means what a parser accepts.

| | ruby-prism 2.1.0 | tree-sitter-ruby 0.23.1 | lib-ruby-parser 4.0.6 |
|---|---|---|---|
| Kind | the official Prism parser (the one in CRuby 3.4+), C library vendored in the crate, Rust typed AST over it | C grammar, same tree-sitter runtime as the other languages | pure Rust, port of the old `parser` gem, targets Ruby 3.1.2 |
| Corpus files with a parse error | 0 of 18,958 | 5 (perl_backrefs, special_global_vars, three json_api_kit files) | 15 (3.2+ anonymous forwarding, `ractors.rb`, others) |
| Probes failing | 0 valid ones; correctly rejects the invalid-in-3.4 `a[&]` | 1: endless method with a command body (`def hi(n) = puts "x"`); wrongly accepts `a[&]` | 2: anonymous argument forwarding (3.2), one more endless-method probe; rejects `a[&]` |
| Parse speed | 124-178 MB/s (all 134) | 7-11 MB/s (all 7.8) | 19-28 MB/s (all 20) |
| Tokens + trivia tile the source | tokens through the C lex callback (`pm_parser_lex_callback_set`, not wrapped by the Rust crate, declared by hand through `ruby-prism-sys`): 14.4 M tokens, comments included, gaps are whitespace only in all 18,969 files; lex+parse 122 MB/s | by the adapter, as for other grammars | tokens recorded by the parser; gaps checked only loosely |
| Mutants: lost to the error (statement-level, median) | AST keeps recovering, but a missing `end` makes the enclosing top-level statement unusable (whole file in the measure used) | median 64% / p75 98% of the file in the enclosing top-level children | file lost (no AST) |
| Panics | 0 | 0 | 0 |
| Build | vendored C (about 3 MB of sources, only `cc`, no libclang: bindings are pregenerated); 25 s cold | grammar compiles in seconds | pure Rust, 13 s |
| Upstream | ruby/prism, MIT, maintained by the CRuby core team, follows each Ruby release | tree-sitter org, MIT, slow | MIT, last release targets 3.1, stalled |

Binary-size delta was not measured (the scratch tree was lost before it ran); the vendored Prism archive is 1.7 MB unstripped, so expect about +1 MB stripped.

Findings:
- Prism parses every file and snippet, is 17x faster than tree-sitter and 6x faster than lib-ruby-parser, and is the reference for what is valid: whatever it rejects, CRuby rejects (it rejected `a[&]`, which 3.4 forbids, where tree-sitter accepts it).
- lib-ruby-parser lacks Ruby 3.2+ syntax and is unmaintained: out.
- tree-sitter-ruby fails 5 real files and a 3.3 endless-method form, and it is 17x slower.
- The Rust crate exposes the AST, comments, magic comments and `__END__` location, but no tokens. The lex callback is in the linked C library; a 40-line `extern "C"` wrapper (own `pm_parser_new`/`pm_parse` call with the callback set) yields all tokens at no extra cost, so the adapter replay gets its tokens as for Java. Heredoc bodies arrive as tokens in lex order; sort by start.
- Prism's error recovery is the best of the three (it reports several diagnostics and still builds a tree), but the statement-level loss in the mutant measure was not refined; a formatter refuses files with errors anyway.

## 2. Recommendation
**ruby-prism through the adapter replay** (`=2.1.0`, pinned; used for parsing only: no Prism-derived formatter), with tokens taken from the lex callback by a small FFI wrapper and a kind table of the Prism node types (about 150 rows in `grammars/ruby.grammar`). Costs to accept: a vendored C build (the tree-sitter languages already need a C compiler), the hand-written token wrapper, and a pin that moves with Prism releases.

Fallback: none worth taking (tree-sitter-ruby: 5 failing real files, 17x slower).

## 3. Reference tools for later steps
- Format: RuboCop's Layout cops (`rubocop -a --only Layout`, the style most teams run), Standard (RuboCop with fixed config), syntax_tree (`stree`, a prettier-like formatter, closest to a pure formatter), rufo.
- Lint: RuboCop Lint cops (and Style/Naming where file-local).
- No system Ruby: the parity runner needs a Ruby. Options to check in the parity step: a prebuilt portable Ruby (ruby/ruby-builder or traveling-ruby tarballs as a `release` asset) with the gems installed by `gem install --install-dir`, or the `npm`-style kinds are not available for gems. Not tried here.

## 4. Steps
| Step | Builds |
|---|---|
| 1 | `lang-ruby`: Prism adapter (AST spans + lex-callback tokens), lossless tree, provider, `.rb`/`.rake`/`.gemspec`/`.ru`, verbatim formatter, corpus script (the four repositories above), bench inputs, docs |
| 2 | Formatter: RuboCop Layout style, parity runner against `rubocop` / `stree`, style options |
| 3 | Lint: RuboCop Lint cops as pattern rules and families |
