# C/C++ format: path to 85% by mechanism

Current plan. Reference: clang-format 20.1.8, 6,639 files, 85% = 5,644 equal. All counts are upper bounds.

Data: wrap-r8.json (4,597 equal, 2,042 differing, 1,047 needed). Every gap of a differing file is mapped to one mechanism
(by gap name and parent kinds; lambda = a Lambda kind in parent, grandparent or the tokens). A file counts as fixed only
when ALL its mechanisms are fixed (solo + pair + triple cover, greedy by files completed). Numbers are upper bounds: a
mechanism that is built imperfectly leaves files behind, and files also hide gaps behind a first one (the comparator lists
every gap, so this is already counted).

## Cumulative, best order (equal files after each mechanism)

| # | Mechanism | Files touched | Files completed (cumulative) | Equal |
|---|-----------|--------------:|-----------------------------:|------:|
| 0 | now | | | 4,597 |
| 1 | comments outside lists (comment-other: own-line comment indent/join in blocks, directives, declarations) | 509 | 113 | 4,710 |
| 2 | comments inside lists (parameters, arguments, init lists, template args) | 451 | 228 | 4,825 |
| 3 | trailing comment column alignment | 356 | 354 | 4,951 |
| 4 | pointer / reference alignment (parens, directives, unary, conditions) | 427 | 495 | 5,092 |
| 5 | brace placement `brace-own-line-in-source` (non-lambda) | 423 | 584 | 5,181 |
| 6 | short lambda bodies (brace, continuation indent, empty body) | 257 | 721 | 5,318 |
| 7 | continuation indent of non-wrap lines | 318 | 855 | 5,452 |
| 8 | braced list padding | 171 | 936 | 5,533 |
| 9 | column layout of initializer tables | 160 | 1,019 | 5,616 |
| 10 | binary operator spacing | 209 | 1,099 | 5,696 |

5,644 is reached between steps 9 and 10: the first nine mechanisms give 5,616, the tenth (or the sum of the next three
spacing families) passes it. Long tail begins at step 10: after it each mechanism adds under 85 files and the next ones
(spacing-rare 81, escaped-newline align 76, cast-space 78, brace-own-line-in-reference 67, space-before-paren 70) are
each under 3.5 percent of the gap. The first nine are all large (80-140 files each) and sum to 1,019 of the 1,047 needed.
Step 9 alone needs 3 steps of the same family to finish, so the real minimum is all nine plus one tail family.

Order matters little: files hold 1-3 mechanisms, so the cover is not additive; the numbers above are greedy completion
counts. Doing the mechanisms out of order gives the same total after step 9 but a lower curve before it.

## Mechanisms, kit concept, second language, size

1. Comments outside lists: extends the shared comment layer (`comments.rs`, kit row "comments: own-line and trailing") with
   rules for a comment between declaration parts and after a directive. Second language: any with a blocky comment
   (C#, Java, Go, Rust all hit the same joins). Size: 150-250 lines shared, 0 in lang-cpp (rows or options).
2. Comments inside lists: extends the list shapes (`items`, `calls.rs` argument lists, init lists) so a comment between
   elements keeps its line and its column neighbours. Second language: C# and TypeScript argument lists, Rust struct
   literals. Size: 200-300 lines shared.
3. Trailing comment column alignment: extends `align_trailing_comment` / `align_lines` (context.rs, comments.rs
   `gap_before`) from "one run, one column" to clang's rules (alignment across blank-free groups, a comment that no longer
   fits, `AlignTrailingComments` with consecutive-line limits). Second language: Go (gofmt aligns trailing comments in
   blocks), C#. Size: 100-200 lines shared, mostly options in lang.yaml.
4. Pointer and reference alignment: extends `spacing.rs` `pointer_gap` and the `pointer_in` kind sets to more parents
   (ParenGroup, Directive, Unary, Condition). Experience: adding parents blindly lost 550 files, so it needs a real
   declarator-versus-expression decision: the deep parse of C++ must tag declarations in parameters, conditions and
   macro arguments. Second language: Go and Rust types (`&`/`*` spacing), D. Size: 150-250 lines, part in lang-cpp deep
   parse (grammar lookahead, allowed by the lead ruling), part shared.
5. Brace placement `brace-own-line-in-source`: extends the `brace_wrapping` option (layout.rs, lang.yaml `attach` kind
   sets). The gap is constructs the deep parse does not tag (macro-block, class vs function in headers, braces after
   macros). Second language: C# (Allman/K&R), Java, PHP. Size: mostly deep-parse work in lang-cpp (grammar facts, about
   100-200 lines) plus 50 shared.
6. Short lambda bodies: extends `compound`/`block` shapes with a "short body fits, else expand" decision for lambdas as
   call arguments; clang breaks before the lambda argument rather than inside its body. Second language: C# lambdas,
   Rust closures, TypeScript arrows (all want "short body stays inline in an argument list"). Size: 250-400 lines shared
   (printer-level: a break priority between argument and body, so it touches doc/printer fits logic).
7. Continuation indent of non-wrap lines: extends the indentation of continued statements (layout `items`, `chain`);
   second language: C#, Java (all continuation indent options). Size: 100-150 lines shared.
8. Braced list padding: extends `pad_gap` (spacing.rs, kit "padded braces"): the `Cpp11BracedListStyle` rule that
   `{a}` vs `{ a }` depends on the list's use. Second language: JS/TS object padding (`{ a }`), Rust, Swift. Size:
   about 60 lines shared plus kind sets.
9. Column layout of initializer tables: new shared shape (`table`/columns) that pads the cells of a braced list of rows
   to a common width (clang CommaSeparatedList). Second language: YAML/JSON flow tables, Rust arrays and Go composite
   literals (gofmt aligns key: value columns). Size: 200-300 lines shared in a new module with a kit row, 0 in lang-cpp.
10. Binary operator spacing and the other spacing families (the rest): small causes in wrapped or macro contexts, each
    3 to 80 files; long tail.

## Honest read

- Total shared work for steps 1-9 is roughly 1,200-1,800 lines of shared Rust and 200-300 of lang-cpp. That is 6-9
  steps of the size of the current batch, not one.
- The first reachable block with the best ratio is 1-3 (all comment mechanisms): 354 files for about 500 shared lines.
- Steps 5 and 6 are the risky ones (parse and printer level); step 4 is risky for regressions (see the 550-file loss).
- 85% is reachable only with all nine; skipping any one of 5, 6, 7 leaves the target out of reach (the long tail
  after step 10 cannot make up more than about 400 files in total).

## Measured so far

- Mechanism 1 (comments outside lists), first batch: a region under `clang-format off` keeps the comments after the marker and the indentation of its first line (`FormatCtx::verbatim_region`); a comment at column 0 right above a directive stays at column 0 (`leading_comments_doc_before_directive`). Result 4,597 -> 4,606 (+9 of the 113 files the mechanism could complete). The rest of the "comment-other" files are comment lines inside statements that clang wraps differently (continuation indent, `template(...)(requires ...)` macro heads, trailing `//` forcing a break): they depend on wrapping, not on the comment layer. Steps 2 and 3 have the same dependence: parameter lists with comments are laid out by clang one parameter per line with aligned trailing comments.
