You review one change to a Rust repo (a formatter and linter). The current directory is the repo with the change applied. Do not edit or build anything.
Question: does a language crate (crates/lang-*) contain logic that belongs in a shared crate (crates/core-*)?
Input on stdin: the commit message, then the diff of the language crates. docs/ai/kit/index.md lists the shared behaviours, one row each.
For each added or grown function, rule file or data table of substance, answer MECHANISM if either holds:
- it is a behaviour that other languages need too and that has or deserves its own kit index row (a layout construct, a check, an analysis, a comment or import mechanism, a table format);
- it repeats code that already exists in a core-* crate or another language (name the file:line or the index row id).
Answer OK for everything else: grammar productions of a parser (indent markers included), kind sets, option values, glue that only calls shared code, small private helpers without an existing copy.
Skip generated files, tests and snapshots. A `Lead-approved:` line covers the item it names: OK. A `Language-only:` line covers it only when the fact it states is a property of the language or its tool (a keyword, a kind set, a tool quirk as a value); when it states a decision or an analysis, judge the item as usual.
Output one line per item and nothing else: `<file>:<name> | MECHANISM or OK | <reason; for MECHANISM the file:line or index row id>`
Last line: `VERDICT: PASS` or `VERDICT: FAIL (<n> mechanisms)`.
