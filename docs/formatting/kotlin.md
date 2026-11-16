# Kotlin (`lang-kotlin`)

## Parser
[ktrs-parser](https://github.com/Hexay/ktrs) 0.5.1 (MIT OR Apache-2.0) with `ktrs-syntax`, pinned to the exact version in the root `Cargo.toml` (dependabot ignores them; a bump is a deliberate change of `grammars/kotlin.grammar` and `crates/lang-kotlin/src/adapter.rs`, followed by the conformance run). It is a port of the Kotlin compiler's PSI parser, so node and token names are the compiler's (`FUN`, `VALUE_PARAMETER_LIST`, `LONG_STRING_TEMPLATE_ENTRY`), the ones ktlint's rules are written over. `.kt` files are source files, `.kts` (Gradle Kotlin DSL, scripts) are scripts; the file id decides.

`adapter.rs` walks ktrs's flat tree once and hands tokens and nodes to the shared `replay`:
- a node is the row of `grammars/kotlin.grammar` named like its PSI type in Pascal case (`FUN` is `Fun`); a PSI type without a row is transparent;
- a token is ours by its kind: every keyword (hard, soft, modifier) is `Keyword`, strings are `StringTemplate` nodes of `OpenQuote`, `StringPart`, `EscapeSequence`, `$` / `${` entries and `ClosingQuote`, and other punctuation is the grammar token with that text;
- whitespace, comments and KDoc are `replay`'s gaps (`//`, nested `/* */`, a `#!` first line) and follow the shared attachment rule; KDoc is a `Comment` like the others, its structure is not in the tree;
- an `ERROR_ELEMENT` is an `ErrorElement` node and a parse error with ktrs's message; tokens the parser left out of its tree become `Unknown` tokens with an error.

What ktrs needs from its caller, done in the adapter:
- it requires `\n` line breaks: the text is normalized and every offset mapped back (`crs`, a binary search), so a CRLF file keeps its line breaks;
- it recurses once per nesting level and aborts the process on overflow, so the parse and the walk run on a thread with a 256 MB stack, and a scan refuses text nested deeper than 2,000 brackets or 2,000 prefix operators; it is exponential in nested annotation arguments (`@A(@A(...))`, 4 s at 22 levels), so more than 12 are refused; a panic is caught; a file over 16 MB is not parsed. All of these leave the file as one `Unknown` token and one error, which the formatter and the linter refuse.

Failure modes of the parser itself: it follows the compiler of Kotlin 2.4.20 and rejects the newest experimental syntax of the compiler's master (union types, `error` as a modifier); it is quadratic on a chain of tens of thousands of binary operators.

## Options
<!-- generated:options -->
| Option | Values | Default | Description |
|---|---|---|---|
| `indent_size` | int 1-16 | `4` | Columns per indentation level (also the width of a tab). |
| `line_width` | int 20-1000 | `140` | Preferred maximum line width in columns. |
| `indent_style` | `space`, `tab` | `space` | Indent with spaces or tabs. |
| `final_newline` | bool | `true` | End the file with a newline. |
| `end_of_line` | `auto`, `lf`, `crlf` | `auto` | Line break style of the output: `lf`, `crlf`, or `auto` to keep the file's first line break. |
<!-- /generated -->

## Formatting
Verbatim for now: every file prints as written.

## Conformance
`bash tests/conformance/fetch-kotlin-corpus.sh`, then `cargo test --profile corpus -p lang-kotlin --test integration -- --ignored --nocapture conformance`: kotlinx.coroutines, ktor, nowinandroid, ktlint and the Kotlin standard library, `.kt` and `.kts` (6,632 files, 20 s in the `corpus` profile): every file parses without errors, prints back byte for byte and satisfies the trivia rule; the 322 compiler PSI parser test inputs (`testData/psi`) that hold deliberate errors are reported, not required to parse. An unclosed block comment is an error.
