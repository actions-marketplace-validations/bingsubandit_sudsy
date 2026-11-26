# Parity target number

Every parity run reports one number, the **target**: the share of the reference tool's result that a user of the tool can get from Sudsy. 100% is the goal. Only reviewed intentional cases leave the count; everything else counts against Sudsy, including style choices without an option and every gap. There is no second, unadjusted number.

## What counts
- **A result a user can get counts in Sudsy's favour.**
  - A token rewrite the formatter leaves to a lint rule's fix is compared as `sudsy fix`, with only the rules that mirror the tool's rewrites on (the terraform run works this way).
  - A style difference with a named option is compared with the tool's preset setting that option.
- **Style choices with no option, and gaps, count against Sudsy.** They leave the count by getting an option or a fix rule, or by being fixed. No intentional flag exists for them.
- **Only four kinds of case are intentional.** Only the lead or the user adds one; each is pinned so it cannot hide a later change:
  1. **Invalid input.** The file leaves the comparison and is printed in its own group with a count.
     - The tool rejecting the file is evidence enough.
     - When Sudsy rejects a file, the language's reference parser must reject it too (`reference_parser` of the tool's entry); otherwise it is a Sudsy bug, printed as `REVIEW`, and counts.
  2. **The tool is wrong.** Its output changes meaning, drops content or is unstable on its own output.
     - A named entry with `intentional: tool-wrong` and `pins`, the hash of Sudsy's output for the file as reviewed.
     - When Sudsy's output for the file changes, the file is compared again and printed as `REVIEW`; it counts until the lead pastes the new pin (`scripts/parity.sh --pin <tool>`).
  3. **Lint findings where the tool is wrong.** The same entry and flag, pinned per finding (file, line, rule), not per file.
  4. **Out of scope.** `wontfix` catalog entries for removed or deprecated rules and for domains Sudsy does not cover (package manifests, a tool's own config, CLI modes). A `wontfix` that reads as "not built yet" is `planned` or `deferred` and counts.
- **A lint rule the corpus cannot judge** (coverage runs). The tool's findings of the rule change with what the corpus lacks (the references of its projects): listed under `not-comparable` in `differences.yaml` with the evidence per rule (the counts of the tool's own build and of the corpus run on the part of the corpus that compiles completely), left out of the target, printed with their count in every run. The ids with no finding in that part stay in. If the listed rules are more than a third of the set's findings, the corpus gets real references instead.
- **`formatter` and `option <lang>.format.*` catalog entries need a proof run.** A coverage run of a tool with such entries formats a copy of the corpus with `sudsy format` (or `sudsy fix`, as above) under the tool's preset and runs the tool again. A finding of a `formatter` id that remains counts as missing. A tool without a corpus keeps its `formatter` entries out of any number (the counts table shows `-`).
- **No silent filters.** Every input filter (non-UTF-8 files, `skip`, `skip_with`, a tool rejecting a file, flags that turn off features of the reference tool) prints its count in every run. A flag is a named gap (`flags` of `differences.yaml`) with the number of fixtures it applies to, until the feature exists or the project options are mapped.

Lint position: a finding is covered only when the mapped rule reports it on the same line (the column is not compared). In the target, a report on another line that is a reviewed `tool-wrong` entry (pinned per finding) leaves the count.

## What a run prints
Per run: the target as `target N% (identical of left)` (format) or `target N% (hit of left, same line)` (coverage), then the intentional groups with their counts (`invalid-input`, `tool-wrong`, `REVIEW`, out of scope, formatter findings counted as missing) and every filter count. A tool whose `coverage` entry in `scripts/parity_tools.json` has `heldout` (project folders under the corpus the rules were not tuned on) also prints `held-out projects, target ...`, the same number over those projects only.

## Where the number is kept
- **Format:** `ratchet` (least identical files) and `target-files` (the target's files: all minus the intentional ones) per group in `tests/parity/<tool>/differences.yaml`. A run fails below `ratchet` or when `target-files` is stale; the number is `ratchet` over `target-files`. `second-format` (most files that change on a second format) is a separate ratchet, target 0.
- **Lint:** `target` in the same file, a ratchet that may only go up.
- **No reference formatter:** `scripts/parity.sh stability <group>` (`scripts/parity_stability.py`, the `stability` section of `scripts/parity_tools.json`) is the number: the share of the corpus files that parse, format with the safety check passing, are unchanged by a second format and are laid out by the engine rather than copied through (`core_format::copied_through_files`). Its ratchets are `ratchet` and `target-files` in `tests/parity/stability/differences.yaml`; no input is left out.
- **README table "Language support"**, generated by `cargo xtask codegen` from these numbers (`xtask/full/src/support.rs`, rows in `xtask/full/support.yaml`; `codegen --check` fails when it is stale). One row per language; format and lint cells list each reference tool as a bar with its target, `not measured` where no run exists. A tool folder's local `fixtures` group never counts (a few files of the folder, not real code): a format leg needs a corpus group (`real-world`, named in `groups` of `support.yaml`), else it reads `not measured`. `Ready` means the primary format and lint tool are at or above the CLAUDE.md bar; `In progress` below it; `Lint only` for a language Sudsy does not format. A lint leg with no reference linter reads `N rules · no reference tool` and never decides `Ready`. A tool with a `note` in `support.yaml` (what its comparison leaves out) adds a `Notes:` line under the table. Reproduce a number with `scripts/parity.sh <tool>` or `scripts/parity.sh coverage <tool>`.
- **Counts table and queue:** the Target column of [../parity/README.md](../parity/README.md) and the ranked causes of [../parity/queue.md](../parity/queue.md) are generated from the same files.

Tests: `scripts/parity.sh --test` runs `scripts/test_parity.py` (a pinned file whose output changed, a filter count, a `fix`-mode comparison); it is part of the gate.
