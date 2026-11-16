# Design: PHP

Goal: formatter and linter for `.php` (and `.phtml`, `.inc`) files, PHP 8.0 to 8.4 syntax and the 8.5 syntax the parser already knows. The parser approach is the user's decision (CLAUDE.md); this note measures the candidates and recommends one. Nothing is built yet.

## 1. Parser comparison (measured, not committed)
Linux arm64 Graviton, 8 cores, rustc 1.99.0, release with LTO, parse of the whole file from memory, warm, best of 5 rounds, no adapter. Corpus: 10,390 `.php` files (about 100 MB, pinned tarballs): laravel/framework `b65b5e3`, symfony/symfony `a9bd44f` (6,209 files, including its test fixtures), WordPress/WordPress `6558c09`, composer/composer `5147ea1`. Probes: 46 hand-written files of 8.0 to 8.4 syntax and 5 of 8.5 (property hooks, asymmetric visibility, enums, readonly, DNF types, first-class callables, attributes, heredoc/nowdoc, alternative syntax, mixed HTML, keyword-named members, `__halt_compiler`). Mutants: one `)`, `}` or `{` deleted at a pseudo-random place of 2,536 valid files (500 B to 40 KB). Binary: stripped release with only the parser behind a `main` (empty `main`: 0.33 MB). No `php` is installed, so "valid" means what a parser accepts; the error-free files of the corpus are the valid set.

| | mago-syntax 1.59.0 | tree-sitter-php 0.25.1 |
|---|---|---|
| Kind | Rust, hand-written lexer + parser, typed AST (about 226 node types), trivia as a separate list | C grammar (`LANGUAGE_PHP`, HTML text around `<?php`), same tree-sitter runtime as the other languages |
| Corpus files with a parse error | 0 of 10,390 | 8 of 10,390 (`const VOID`, `function readonly()`, `clone $a ??= ...` and five more: heredocs, a `"\0$x"` string, a non-UTF-8 class name) |
| Probes failing (46 + 5) | 0 | 3 + 2: nested `list('a' => $x, [$y, , $z])`, keyword-named class constants and methods (`const VOID`, `const ENUM`), text after `__halt_compiler();`; 8.5 `(void)` cast and `final` promoted property |
| Parse speed | 125-206 MB/s (Symfony 206, Laravel 160, WordPress 141, Composer 125) | 10-13 MB/s (Symfony 11.6, Laravel 12.7, WordPress 11.5, Composer 9.9) |
| Stripped binary added | +0.33 MB | +1.18 MB (both grammars of the crate) |
| Tokens + trivia tile the source | yes, all 10,390 files (lexer yields whitespace, comments, inline HTML) | by the adapter, as for other grammars |
| Mutants: statements lost to the error (median / p75 / p90) | 1.5% / 38.6% / 95.6% of the file | 0% / 1.0% / 11.6% of the file in `ERROR` nodes |
| Deep nesting (`[[[...]]]`, `((( )))`) | graceful error from 1,000 levels (depth guard, no crash); 200,000-term `+` chains and 20,000 `elseif`s fine | all of 20,000 fine |
| Dependencies | 103 crates, among them rayon, notify, globset, blake3, serde, schemars (through `mago-database`, which `mago-syntax` requires for `FileId`); build 20 s | `tree-sitter` runtime only; the C grammar compiles in seconds |
| Upstream | carthage-software/mago, MIT OR Apache-2.0, 3.6k stars, 156 releases since Apr 2025 (weekly), 100+ contributors | tree-sitter org, MIT, 24 releases, last 0.25.1 in Oct 2026 (8.5 `clone` with grammar), 37 contributors |
| Toolchain | `rust-version` of each release is the newest stable at its date: 1.60.x needs 1.100, 1.59.0 is the last for our pinned 1.99.0 | any |

Findings behind the table:
- mago passes every file and probe tree-sitter fails, including the 8.4 syntax (property hooks, `new Foo()->m()`, asymmetric visibility) and the 8.5 pipe operator, `clone(..., [...])`, `(void)`. tree-sitter's failures are all on keywords used as names, which real code (WordPress, Symfony) contains.
- Error recovery is the one place tree-sitter is better: a mago error drops the whole statement it occurs in (a class is one statement, so an error in a method loses the class), while tree-sitter keeps most of the file. For the formatter this does not matter (a file with errors is refused); for lint diagnostics and the language server the error position is reported either way, and rules do not run on a broken file today.
- mago's parser has no panics on the corpus, mutants or deep probes (a `catch_unwind` is still cheap insurance).
- The mago API is a typed AST with `Walker` traversal and spans on every node; the adapter (`provider_api::adapter::replay`) needs node spans in pre-order, tokens from `mago_syntax::lexer::Lexer` (tokens include whitespace and comments and tile the file) and a kind table of about 226 rows in `grammars/php.grammar`, like Java and Kotlin.
- Not measured: the adapter itself, memory (mago's arena: the whole program is one bump allocation).

## 2. Recommendation
**mago-syntax through the adapter replay**, pinned exactly (`=1.59.0` today; releases are weekly, their MSRV moves with stable, and a bump touches the kind table and the adapter only), with `mago-syntax` used for parsing only (`mago-formatter`, `mago-linter` and `mago-analyzer` are not dependencies: layout and rules are ours, they are a behaviour reference). It is 12x faster, 3.5x smaller, accepts everything in 10,390 real files and the 8.4/8.5 probes, and the arena AST gives exact spans. Costs to accept: a 103-crate dependency tail through `mago-database` (rayon, notify, globset; to check against `cargo deny` and the binary budget before the first step lands, a fork or vendored subset of `mago-syntax` is the fallback), coarse error recovery, and the toolchain pin moving in step with mago (a release newer than 1.59.0 needs rustc 1.100).

Fallback: tree-sitter-php (`tree-sitter-php = "0.25"` works with our `tree-sitter` 0.27 through `tree-sitter-language`), at 8 failing real files, no keyword-named constants and 12x slower parsing.

## 3. Reference tools for later steps
- Format: PHP-CS-Fixer (`php-cs-fixer.phar`, v3.95.31 on GitHub releases, rule sets `@PER-CS` / `@PSR12` / `@Symfony`) is what teams run; PER Coding Style is the shared standard to implement as the default style. Prettier with `@prettier/plugin-php` (0.26.0, Node) is the other common formatter and fits the existing `prettier` parity entries; `mago fmt` (a single Rust executable) is a third reference that needs no PHP.
- Lint: PHPStan (`phpstan.phar` 2.3.6, type analysis; most of its rules need whole-project types and stay out of reach), PHP_CodeSniffer (`phpcs.phar` 4.0.4, PSR-12 and style sniffs: the nearest match to Sudsy's file-local rules); `mago lint` as a Rust executable reference.
- No system PHP: PHP, PHP-CS-Fixer, PHPCS and PHPStan are phars that need a PHP 8 CLI. Static builds exist (`dl.static-php.dev/static-php-cli/common/php-8.4.23-cli-linux-{x86_64,aarch64}.tar.gz`, about 12 MB, also macOS and Windows assets; the `common` build includes `phar`, `tokenizer`, `mbstring` as far as its listing shows). That fits the parity runner's `release` install kind (URL template, per-platform `assets`, archive) for the interpreter, with the phar a second `release` asset run as `{prefix}/php php-cs-fixer.phar fix --dry-run --diff`; the Prettier plugin fits the `npm` kind (`with: [@prettier/plugin-php]`). Downloading and running these was not tried here (the session was not allowed to), so the extension list and the run on the corpus above are to be checked in the parity step.

## 4. Steps
| Step | Builds |
|---|---|
| 1 | `lang-php`: adapter over mago-syntax, lossless tree, provider, `.php`/`.phtml`, verbatim formatter, corpus script (the four repositories above), bench inputs, docs |
| 2 | Formatter: PER-CS layout, `scripts/parity.sh format <tool>` (tool chosen then), style options |
| 3 | Lint: PHPCS/PHPStan-style file-local rules as pattern rules and families |
