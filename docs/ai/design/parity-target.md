# Parity target number plan

Every parity run reports two numbers (user decision 2026-11-19):
- **raw**: what is measured today;
- **target**: the number the work aims at, 100% meaning that a user of the reference tool can get its result from Sudsy.

The target leaves out only reviewed intentional cases. Everything else counts against Sudsy, including style choices without an option and every gap.

## Rules for the target number
- **A result a user can get counts in Sudsy's favour.**
  - A token rewrite the formatter leaves to a lint rule's fix is compared as `sudsy fix`, with only the rules that mirror the tool's rewrites on (the terraform run already works this way).
  - A style difference with a named option is compared with the tool's preset setting that option.
- **Style choices with no option, and gaps, count against Sudsy.**
  - No intentional flag exists for these.
  - They leave the count by getting an option or a fix rule, or by being fixed.
- **Only four kinds of case are intentional.** Only the lead or the user adds one; each is pinned so it cannot hide a later change:
  1. **Invalid input.** The file is excluded from the comparison and printed in its own group with a count.
     - The tool rejecting the file is evidence enough.
     - When Sudsy rejects a file, the language's reference parser must reject it too; otherwise it is a Sudsy bug and counts.
  2. **The tool is wrong.** Its output changes meaning, drops content or is unstable on its own output.
     - It is a named entry with `intentional: tool-wrong` and the hash of Sudsy's output for that file, as reviewed.
     - When Sudsy's output for the file changes, the file is compared again and reported for review.
  3. **Lint rules where the tool is wrong.** The same entry and flag, pinned per finding (file, line, rule), not per file.
  4. **Out of scope.** `wontfix` catalog entries for removed or deprecated rules and for domains Sudsy does not cover (package manifests, a tool's own config, CLI modes). A `wontfix` that reads as "not built yet" is `planned` or `deferred` instead, and counts.
- **`formatter` catalog entries need a proof run.** They leave the lint denominator only after a run per tool that:
  - formats the coverage corpus with `sudsy format` (or `sudsy fix`, as above);
  - runs the lint tool on the result.

  A finding of a `formatter` id that remains counts as missing. StyleCop's `layout_*` check is the model. Until a tool has the run, its `formatter` findings stay in the denominator.
- **No silent filters.** Every input filter (non-UTF-8 files, `skip`, `skip_with`, a tool rejecting a file, flags that turn off features of the reference tool) prints its count in every run.
- **Every run prints three things:** raw %, target %, and per intentional group its count.

Lint coverage keeps the CLAUDE.md bar: the mapped rule is reported in the same file; the position is not compared.

## Phases

### Phase 1: runner and data
- **The `differences.yaml` entry form gains the `intentional` field.** Allowed values: `invalid-input`, `tool-wrong`; `tool-wrong` entries carry a pinned output hash.
- **The lint named-difference form gains the same field**, pinned per finding.
- **`parity_format.py`, `parity_lint.py` and `parity_coverage.py` compute and print raw and target**, the intentional groups and their counts, and every filter count. A pinned entry whose Sudsy output no longer matches its hash is reported as `REVIEW` and counted against Sudsy.
- **`scripts/parity.sh --pin <tool>`** prints the hashes for the lead to paste after review; the runner never writes them itself.
- **Docs:** [../development/parity.md](../development/parity.md) and [../parity/README.md](../parity/README.md) describe the two numbers. The generated counts table gains a target column.
- **Gate:** a fixture test of the runner covers:
  - a pinned file whose output changed;
  - a filter count;
  - a `fix`-mode comparison.

**Checkpoint 1:** report the new output of one format run and one coverage run, before data changes.

### Phase 2: classify today's cases
Every `choice` in `differences.yaml` and every `wontfix` in the catalog gets one of these outcomes. The inventory of 2026-11-19 is the starting list.
- **Intentional `invalid-input`:**
  - files the tool rejects (sqlfluff, php-cs-fixer, ktlint);
  - taplo `taplo-rejects`;
  - terraform `semantic-error`;
  - XML `not-xml`;
  - PowerShell `not-powershell`;
  - GraphQL `invalid-graphql`, only after graphql-js confirms each file is invalid.
- **Intentional `tool-wrong`:**
  - Markdown `toc-indent`;
  - taplo `comment-dropped`;
  - XML `width-accounting`;
  - prettier-angular `non-idempotent`. Sudsy stops copying Prettier's unstable output: the formatter change, the `second-format` allowance goes to 0.
- **Intentional lint `tool-wrong`:** the 4 checkmake differences (`empty-recipe`, `target-specific-variable`, `double-colon-rule`, `multi-target-rule`).
- **`fix`-mode comparison:**
  - shfmt `backticks` (rule `no-backticks`);
  - terraform `interpolation-in-list`. This needs a rule option, lists too, that the terraform preset turns off. It is a named option in the rule's schema, not a preset branch.
- **Counted as gaps** (the entry changes from `choice` to `gap`): the remaining style and token-spelling choices, for example:
  - shfmt `expr-kept`, `case-last-arm`, `heredoc-tabs`, `comment-column`, `empty-file`;
  - taplo `multiline-string-array`, `inline-table-array`, `empty-file`, `crlf-in-string`;
  - Markdown `tab-blocks`, `code-trailing-blanks`;
  - terraform `token-rewrite`;
  - XML `text-as-written`;
  - buf `empty-statements`;
  - PowerShell `backtick-scriptblock`;
  - dotnet-format `initializer-layout`;
  - actionlint `self-hosted-labels`;
  - hadolint `user-variable`;
  - stylelint `tailwind-at-rules`, `element-id`.

  An option for one of them is its own later step, ranked by file count.
- **Relabelled catalog entries:** `wontfix` entries that read as not built become `planned` or `deferred`:
  - eslint's "deprecated formatting rule with no formatter equivalent";
  - Prettier `proseWrap`, `rangeStart`/`rangeEnd`;
  - PSScriptAnalyzer `PSUseCorrectCasing`;
  - sqlfluff `JJ01`, `TQ01`-`TQ03`.
- **Reference-tool flags that turn features off** (gjf `--skip-javadoc-formatting --skip-removing-unused-imports --skip-reflowing-long-strings`, Prettier `--no-config`, ruff `skip_with: .options.json`):
  - keep the flag on the raw run, and record why it is there;
  - each flag is a named gap with a count, until the feature exists or the project options are mapped.
- **Check whether `deferred` entries leave the coverage denominator.** If they do, they go back into it for the target number.

**Checkpoint 2:** report the classification as one table (case, outcome, count), before landing data changes.

### Phase 3: `formatter` proof runs
- **Add the proof run to `parity_coverage.py`** as a mode (`--proof`), for each tool with `formatter` entries and a coverage corpus.
- **Record per tool** the remaining findings of `formatter` ids, and count them in the target.
- **Tools without a corpus** keep their `formatter` entries out of any number; their counts table shows `-`.

### Phase 4: numbers
- Run every format and coverage run.
- Record raw and target per tool in the counts table and in [../development/parity.md](../development/parity.md).
- The ratchets of `differences.yaml` stay on raw identical counts.

### Phase 5: fix what the inventory found
Each item below is its own landing, ranked by the number of files or findings it moves.
- **Style and token gaps from Phase 2 get options or fix rules** (named, user-settable, set by the tool's preset): shfmt `comment-column`, `heredoc-tabs`, `case-last-arm`, `expr-kept`; taplo `multiline-string-array`, `inline-table-array`; terraform `token-rewrite`; buf `empty-statements`; XML `text-as-written`; Markdown `tab-blocks`, `code-trailing-blanks`; the rest of the Phase 2 gap list. Single-file trivia (an `empty-file` choice) may stay counted when an option costs more than it moves; say so in the commit.
- **Reference-tool features that are turned off get built:**
  - gjf Javadoc formatting, unused-import removal and long-string reflow (then the flags go);
  - mapping the projects' own Prettier options in the real-world JS, CSS and HTML groups (`project-options`), then `--no-config` goes;
  - ruff fixtures with `.options.json` run with those options.
- **Unclear items are investigated and resolved:**
  - the four stylelint rules that "differ from stylelint" (`declaration-block-single-line-max-declarations`, `function-url-quotes`, `selector-max-combinators`, `selector-type-case`): each becomes a fix or a `tool-wrong` entry;
  - the JS class-header line comment that moves into the body: fix or `tool-wrong`.
- **Tools with no measured share get a coverage corpus where one can run:** angular-eslint, biome, graphql-eslint, stylelint-scss, stylelint-less, eslint-plugin-jsonc, taplo, terraform, detekt, PMD, PHPCS. C# analyzers and clang-tidy stay unmeasured (no compiling corpus; no C/C++ lint yet), named as such in the README.

## Done when
Every parity run prints raw, target and the intentional groups with counts. No `choice` entry is left without an `intentional` value or a change to `gap`. Every `formatter` entry with a corpus has a proof result. The counts table shows both numbers. Every Phase 5 item is landed or named as a gap with its count. Docs describe current state.
