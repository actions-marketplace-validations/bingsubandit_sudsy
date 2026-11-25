# Design: Makefile format evidence

The Makefile formatter ([../formatting/makefile.md](../formatting/makefile.md)) predates [../development/no-reference-tool.md](../development/no-reference-tool.md). This page holds the evidence that page asks for: sources, corpus, comparison with the one partial tool, decision table, agreement, stable. Reproduce with `python3 -I scripts/evidence.py makefile <sudsy> <work-dir> [<mbake>]` (the shared harness; the facts of the language are `scripts/evidence_makefile.py`; it is read-only on the corpus and prints every table below) and `scripts/parity.sh stability makefile`. No formatter change is part of it.

## Sources
Looked up 2026-11-25 (GitHub API, PyPI, npm, crates.io, Homebrew, VS Code Marketplace, conform.nvim, nvim-lint and mega-linter files):

| Tool | Repository | Last push | Stars | Use | Changes | Runs on Linux here |
|---|---|---|---|---|---|---|
| mbake 1.5.0 | EbodShojaei/bake | 2026-11-09 | 767 | PyPI 73k downloads a month; listed by conform.nvim and awesome-formatters; a pre-commit hook; VS Code extension (9k installs) | blanks around assignment operators and after the colon, recipe tabs, blanks before backslashes, blank lines, `.PHONY` grouping; no conditional or continuation indent; no parse (a regex tool) | yes (Python) |
| makefmt 0.0.6 | NazmusSayad/makefmt | 2026-10-13 | 1 | npm 86 downloads a month; created 2026-10-12 | aligns assignments, indents conditionals, formats recipes as shell | not tried: no usage |
| makefmt 0.0.3 | donaldgifford/makefmt | 2026-09-17 | 0 | 768 release downloads | not read | not tried: no usage |
| checkmake 0.3.2 | checkmake/checkmake | 2026-11-23 | 1,220 | 888k release downloads; nvim-lint | lint only | yes (the lint leg) |
| unmake 0.0.27 | mcandre/unmake | 2026-11-23 | 120 | crates.io 33k downloads; nvim-lint | lint only | not needed |
| autotools-language-server | Freed-Wu | 2026-07 | | PyPI 1.7k a month | language server for `Makefile.am`, `configure.ac`; no formatter | not tried |
| mega-linter, pre-commit hooks | | | | the Makefile linter of mega-linter is checkmake | no formatter | |

Unreachable: GitHub code search (needs a login), so the count of `.pre-commit-config.yaml` files that name mbake is not known; the VS Code Marketplace gives installs only for the extension of mbake.

Verdict: mbake is the only maintained, used formatter (two orders of magnitude over the others) and is a partial tool: it normalises blanks and blank lines and leaves indentation of conditionals and continued lines as written. It is the comparison tool of the format leg, not a reference that decides layout, and it is not registered as a parity leg. Where it and the corpus disagree with Sudsy, the corpus decides.

## Corpus
1,021 files of 26 projects, pinned by commit: `tests/conformance/fetch-makefile-corpus.sh` (redis, git, lua, prometheus, kubernetes, docker/cli: the 59 files of the conformance corpus) and `fetch-makefile-evidence-corpus.sh` (20 more repositories, chosen before any run: FFmpeg, vim, micropython, postgres, openwrt, ocaml, node, neovim, cockroach, etcd, helm, containerd, terraform, grafana, golangci-lint, numpy, sphinx, u-boot, buildroot, cpython). Every `Makefile`, `GNUmakefile`, `makefile`, `*.mk` and `*.mak`, except vendored and test inputs; the kernel-style trees (u-boot, buildroot, openwrt) are cut to their build system. Downloaded files are only read: the harness and the formatter never run `make`.

Split: the 26 projects are shuffled with seed 20261127 (`random.Random`); the first 13 decide, the other 13 are held out. Deciding: FFmpeg, cockroach, containerd, etcd, helm, kubernetes, micropython, node, numpy, postgres, prometheus, redis, u-boot. Held out: buildroot, cpython, docker-cli, git, golangci-lint, grafana, lua, neovim, ocaml, openwrt, sphinx, terraform, vim.

## Against mbake
mbake 1.5.0, no config, run on stdin with its own home folder. 795 files are accepted by both tools (Sudsy refuses 225, mbake 2). Files with equal output:

| Files | Held out (278) | All (795) |
|---|---|---|
| equal | 167 (60.1%) | 512 (64.4%) |
| equal when indentation is left out | 208 (74.8%) | 639 (80.4%) |
| and blanks inside lines and blank lines left out | 278 (100%) | 793 (99.7%) |
| left unchanged by Sudsy / by mbake | 84 / 96 | 354 / 420 |

Causes of difference (all 795 files; `[indent]` and `[blank]` are the two classes above):

| Cause | Occurrences | Files |
|---|---|---|
| `[indent]` conditional body or comment: Sudsy indents, mbake keeps | 3,461 | 184 |
| `[indent]` continued line | 1,428 | 67 |
| `[indent]` other | 665 | 31 |
| `[blank]` blanks around an assignment operator | 579 | 81 |
| `[blank]` blank lines added or removed | 305 | 92 |
| `[blank]` blank before the backslash | 219 | 22 |
| `[indent]` recipe line (tab) | 74 | 12 |
| `[blank]` blanks inside a line | 35 | 15 |
| `[indent]` mbake indents, Sudsy does not | 10 | 7 |
| `[blank]` trailing blanks | 5 | 3 |
| lines added, removed or rewritten | 4 | 2 |

(11 causes occur; the 15 largest are all of them.) 99.7% of the files differ only by indentation or by blanks. Indentation is the difference: Sudsy indents the body of a conditional by 2 spaces per level, mbake keeps what is written, and so does most of the corpus (below).

## Decision table
Variants are counted per project on the deciding half (one vote per project; a project votes its most frequent variant). Agreement is the share of held-out occurrences that already have the form Sudsy writes. "Rule" is the result of section 3 of the no-reference-tool page: default at 80% of the projects, default and option at 60 to 80%, as written below 60%.

| Choice | Votes (deciding) | Rule result | Sudsy writes | Agrees | Held-out occurrences with it |
|---|---|---|---|---|---|
| blanks around an assignment operator | 10 one each side, 2 several blanks, 1 none | 77%: default and option | one each side | yes | 4,395 of 6,663 (66.0%) |
| alignment of assignments | 11 not aligned, 2 aligned | 85%: default | not aligned | yes | 1,156 of 1,181 (97.9%) |
| blank before the colon of a rule | 13 none | 100%: default | none | yes | 3,548 of 3,655 (97.1%) |
| blank after the colon of a rule | 13 one | 100%: default | one | yes | 2,726 of 2,820 (96.7%) |
| blanks between the words of a rule line | 13 one | 100%: default | one | yes | 2,865 of 2,868 (99.9%) |
| recipe line indent | 13 one tab | 100%: default | one tab | yes | 4,180 of 4,190 (99.8%) |
| conditional body indent (outside a recipe) | 8 none, 3 tab per level (11 projects) | 73%: default none and option | 2 spaces per level | no | 550 of 3,594 (15.3%) |
| conditional in a recipe | 12 column 0 | 100%: default | column 0 | yes | 515 of 547 (94.1%) |
| rule or directive continuation: blank before the backslash | 7 one | 100%: default | one | yes | 1,133 of 1,142 (99.2%) |
| rule or directive continuation: indent | 3 tab, 1 four, 1 two, 1 eight | below 60%: as written | 2 spaces | no | 46 of 1,142 (4.0%) |
| assignment continuation: blank before the backslash | 7 one, 1 several | 88%: default | as written | not decided | 4,404 of 4,474 are one blank |
| assignment continuation: indent | 3 tab, 2 four, 2 tabs and spaces, 1 eighteen | below 60%: as written | as written | not decided | |
| comment: blank after `#` | 13 one | 100%: default | as written | not decided | 4,882 of 5,208 (93.7%) |
| blank lines | 13 at most one | 100%: default | one, two | yes | 6,389 of 6,391 (100%) |
| trailing blanks in the file | 13 none | 100%: default | none | yes | 295 of 317 (93.1%) |
| final newline | 13 yes | 100%: default | yes | yes | 316 of 317 (99.7%) |

`agreement` on the held-out half: 28,114 of 34,827 occurrences (80.7%) pooled over the decided rows; the mean of the rows' project means is 82.4%. Three rows are below 80% on their own and one of them is above the pooled number only because of the others: conditional body indent (15.3%), continuation indent of a rule or directive (4.0%) and the blanks around an assignment operator (66.0%; the other form is `A=b`, 28% of the held-out assignments, redis, node and containerd style, which the option `space_around_assignment` already covers). The rows of the corpus say: do not indent the body of a conditional by default (8 of 11 projects write none, and 3 more use a tab per level), and keep the indent of a continued line as written. A row left "as written" is not counted in the agreement.

Stable (`scripts/parity.sh stability makefile`, debug build with the safety check): 796 of 1,021 files (78.0%). Every file that parses is stable; the 225 that are not fail before the second format:

| Cause | Files |
|---|---|
| the name of a variable holds a reference: `OBJS-$(CONFIG_X) += a.o` (the grammar reads a name as one word without a reference) | 188 |
| NMake directives (`!INCLUDE`, `!IF`), not GNU make | 10 |
| other parse errors: target-specific `export` and `override`, `$(recurse)` in a name, a stray `)`, `\|` in a value | 25 |
| refused by the safety check (the tree swallowed the file after a reference: neovim `Makefile` with `$(subst $() $(),:,...)`, vim `Make_cyg_ming.mak` at `$(CFLAGS:-fno-asynchronous-unwind-tables=)`) | 2 |

Harm: 30 of the 442 changed files, chosen with seed 20261128 and read. No change of meaning in the sample (the trailing blanks bug below is not in it). Visible harm: the body of a conditional is indented where the project writes it at column 0 (17 files: micropython, postgres, git, buildroot, redis, kubernetes), assignments that a project aligns with tabs or blanks lose their alignment (8: postgres `nls.mk`, `NAME\t\t= x`, FFmpeg `tests/fate`), a `.PHONY` list continued with tabs loses them (grpc, 1); the rest is wanted (`A:=b` to `A := b` in openwrt, double blanks).

## Meaning oracle
The only reference for what a Makefile means is GNU make, and it cannot be run on downloaded files: `make -pn` (print the database without running recipes) still expands `$(shell ...)`, evaluates `include` and `$(eval ...)`, and the host has no sandbox (no user namespaces, no container). It is not used. What is checked safely: `evidence_makefile.py` compares the text of every statement before and after the format with blanks removed (a change of any non-blank character fails; this is the safety check of the formatter again, independent of its parse) and flags the strip of a trailing blank on an assignment value. A check with the real make would need a sandbox host first.

## Left out
Bugs found, with inputs, are in [../bugs.md](../bugs.md): the trailing blank of a value, the parse gaps above. Choices of taste for the user: conditional body indent (default none, option for a tab or spaces per level); blanks around an assignment (default one each side, the option exists); keep the alignment of assignments and the indent of continued lines as written (what the corpus does). mbake's `.PHONY` regrouping and its recipe formatting are rewrites, not layout, and are not part of the plan. Lint is a separate leg (checkmake).

## Plan to the bar
The bar is `stable` 100% of the files the measure counts and `agreement` 80% on the held-out half (mbake is a partial tool, so its 85% does not apply). Two landings.

What the measure should not count: the 10 NMake files (`!INCLUDE`, `!IF`: not GNU make). They are left out of the stability group; the corpus is then 1,011 files and 796 of them are stable (78.7%).

Landing 1, parse (grammar fork and the safety check): a variable name with a reference (188 files), the 25 odd parse errors (target-specific `export` and `override`, `$(recurse)` in a name, a stray `)`, `|` in a value), the 2 files the safety check refuses. The causes are disjoint (one per file), so the joint count is the sum: 215 of the 215 files that fail are fixed only when every one of the three groups is, giving 1,011 of 1,011 parsed. Whether the newly parsed files are all stable is not measured until the grammar parses them; the 796 that parse today are all stable, so the expected `stable` is 100% with a few files that need a follow-up fix inside the same landing. Also here: keep the trailing blank of an assignment value (meaning bug) and the glued `A+=b`, `A?=b`.

Landing 2, layout defaults (shared layout code with `lang.yaml` option values): conditional body indent as written by default with an option for a level of indent, and continued lines of rules and directives as written. Agreement is a pooled share of occurrences, so the rows add: 28,114 plus 1,349 (conditional bodies: 1,899 held-out lines with no indent against the 550 now written) plus 1,096 (continuations: 1,142 as written against 46) is 30,559 of 34,827 (87.7%), with the blanks-around-assignment row (66.0%) unchanged and already an option.

Both landings together reach the bar: `stable` 100% of 1,011 and `agreement` 87.7% (80.7% today). Landing 2 alone gets agreement over the bar; landing 1 alone gets `stable` over the bar. Size: landing 1 is the larger (grammar change and its tests on 215 files), landing 2 is two option values and a kind set.
