# C/C++ format: where the files are

Reference: clang-format 20.1.8, 6,639 files. Equal now: 4,668 (70.3%). The bar of 85% is 5,644 (976 more).
The plan does not show a path to 85%: see the verdict at the end.

## Ranking

Data: the gaps of the 1,971 differing files, each mapped to one mechanism by cause, token kinds and parent kinds. A file
counts as fixed only when ALL its mechanisms are fixed. "Bound" is the greedy completion count (files that become equal
when this mechanism and the ones above it are fixed). Files hold 1 mechanism (about 990), 2 (470), 3 (250) or more
(about 260), so bounds are not additive. "Measured" is what a prototype of the mechanism delivered on the corpus.

| # | Mechanism | Files touched | Sole | Bound | Cumulative bound | Measured |
|---|-----------|--------------:|-----:|------:|-----------------:|----------|
| 1 | short bodies: a one-line body that clang joins or splits | 599 | 180 | 216 | 216 | option cases +61 (landed, below); lambda joins wait for a ruling |
| 2 | pointer and reference alignment | 372 | 106 | 146 | 362 | `Line`, directive and function-pointer parents +15 of 146 (10%) |
| 3 | comments outside lists | 400 | 93 | 107 | 469 | first batch +9 |
| 4 | comments in lists (hug list) | 503 | 84 | 142 | 611 | +34 net of 142 (24%), 16 files lost: branch `cpp-hug-list` |
| 5 | trailing comment alignment | 408 | 56 | 142 | 742 | not built |
| 6 | spacing rest (`->` names, directives, strings) | 359 | 87 | 134 | 876 | not built |
| 7 | `) {` and `{` placement | 277 | 34 | 134 | 1,010 | not built |
| 8 | space before paren (gnu, webkit) | 209 | 45 | 103 | 1,113 | not built |
| 9 | comma space (mostly column-aligned tables) | 155 | 51 | 95 | 1,208 | not built |
| 10 | binary operator and bracket spacing | 157 | 38 | 99 | 1,307 | `[a-1]` rows: a few files |
| 11 | continuation indent | 269 | 42 | 105 | 1,412 | not built |
| 12 | other brace shapes (enum bodies, empty bodies after wrapped initializers, `) {` after wrapped signatures) | 223 | 20 | 137 | 1,549 | not built |
| 13 | statement join and split | 150 | 41 | 119 | 1,668 | not built |
| 14 | `> >` and template angles | 104 | 16 | 87 | 1,755 | not built |
| 15 | comment runs | 104 | 0 | 84 | 1,839 | not built |
| 16 | the rest (blank lines, braced padding, bitfield colon, cast space, tables) | about 150 | about 40 | about 150 | about 2,000 | not built |

Joint sets: the comment mechanisms alone complete at most 345 files, the brace mechanisms alone 342, short bodies and
pointer together 362.

## What the measured yields show

- A change that is an option or a kind set in a place the tree already shows converts nearly all of its bound: the
  ref-qualifier and `SCOPE_EXIT` cases gave +49 where the bound of those causes is about 60.
- A change that needs the tree to decide (pointer in `ParenGroup` and `Line`, comments in lists) converts 10 to 25
  percent, and loses files where the same tokens are an expression (`a && b` against `T&& x`).
- Removing the other causes of a file did not raise the yield of the hug list (+34 alone, +37 on top of the other
  changes): the files it loses and the files it does not convert are different from the files the others fix.

## Short bodies: wrapping or option

`Lbrace|Keyword` in `ReturnStatement` (about 200 files) and `Semi|Rbrace` in `Block` (about 350 files, 218 in a lambda)
show clang joining a body onto one line where Sudsy keeps the lines of the source, and the reverse in about 85 files.
Part of it is an option (`short_functions`, `short_lambdas`: `SCOPE_EXIT { g(); };`, a conversion operator with a
ref-qualifier), landed. The rest follows a width choice: clang joins the lambda body of `auto rng1 =` `v |
views::chunk_by([](P a, P b) { return a.first == b.first; });` only because it first broke after `=`, where the source
has `auto rng1 = v | views::chunk_by([](P a, P b) {` and the body on the next lines. In the final prototype state 192
differing files differ only by such joins and the indent gaps they cause (175 with no other gap), 135 of them in lambda
places. The comparison counts the body brace as exact; a ruling on it is open.

## Verdict

The plan does not show a path to 85%.
- At the measured yields the remaining bounds add up to about 300 to 500 files, so 75 to 77 percent.
- The one step that could close part of the gap, the lambda and short-body joins, is about 190 files (73% if all of them
  were fixed) and depends on how clang chooses where to wrap, which is the wrapping the comparison is meant to ignore.
- Everything else is a long tail of causes under 150 files each, most of them parse questions (declaration against
  expression, template against less-than) that need the deep parse to decide, not rows.
- Order of work if it goes on: option and kind-set items first (they convert), enum bodies as a delimited list with a
  trailing-comma rule (about 55 files, needs an `EnumBody` node of enumerators in the deep parse), the hug list with
  its 16 losses fixed, then pointer parents that the tree decides.

## Measured so far

- Comments outside lists, first batch: a region under `clang-format off` keeps the comments after the marker and the
  indentation of its first line (`FormatCtx::verbatim_region`); a comment at column 0 right above a directive stays at
  column 0 (`leading_comments_doc_before_directive`). 4,597 to 4,606.
- Option cases and kind sets (this step, 4,607 to 4,668, 62 files won, 1 lost): a conversion operator with a
  ref-qualifier is a function (`operator T &() & noexcept { }`), `noexcept`, `override`, `final` and `requires` behind a
  `&` are no names (`pointer_qualifier_words`), a `name(args) { }` macro block joins like a function body unless it is a
  loop macro, a body that holds a block of its own or has a directive in its signature stays expanded, `Line`,
  directives and function-pointer groups hold declarator marks, `a[i-1]` gets blanks.
- List hug (comments in lists): a list with a comment at a line break puts its first item on the bracket line and aligns
  the rest under it when every line fits, else hangs. 4,607 to 4,641 (50 won, 16 lost: `Dot|InitList` indent in the
  kgdb files, `Call` indent after comments, the `Binary` comment gap). Not landed, branch `cpp-hug-list`.
