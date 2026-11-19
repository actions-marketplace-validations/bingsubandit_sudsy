# Speed comparison plan

Every language Sudsy supports gets a speed comparison against each tool it replaces. Each comparison is measured by one shared script, shown in one generated README table, and added for a new tool by a data entry only (user decision 2026-11-19).

## Today
- **Four scripts with hand-written tool lists:** `tests/conformance/speed.sh` (C#), `speed-lint-format.sh` (Python, JS), `speed-json-yaml.sh` and `speed-lint.sh`. They cover JS, Python, C#, JSON and YAML.
- **Mixed machines:** 8-core Linux, 12-core, and Windows with 10-15% noise.
- **Numbers typed by hand** into README.md, docs/performance.md and docs/ai/performance/numbers.md.
- **Lint runs on default rule sets,** so the workloads differ (ruff 407 rules against Sudsy's bug-only set).

## Rules
- **One script, data-driven:** `scripts/speed.sh [<tool>...]`, in Python where that is simpler.
  - Every input comes from the tool's entry in `scripts/parity_tools.json`: its install, version, prefix and real-world corpus.
  - A new tool needs only a `speed` key there. No script edit.
  - A tool without a `speed` key is listed as not measured.
- **The `speed` key** gives the tool's directory-mode commands, since a process per file is not how users run it:
  - `format`: the check command on a folder, e.g. `prettier --check {dir}`;
  - `lint`: the same for lint, e.g. `ruff check --isolated --no-cache {dir}`;
  - `corpus`: the parity corpus group it runs on, with a copy count when the corpus is small.
- **Same work on both sides:**
  - Format: `sudsy format --check` with the preset that mirrors the tool.
  - Lint: `sudsy lint` with the preset that mirrors the tool (`preset:<lang>/<tool>`, the preset the coverage run uses), so both run the same rule set.
  - A tool with no mirroring preset is not measured for lint until it has one.
- **Measurement:**
  - A new process per run, caches off (each tool's no-cache flag in its `speed` command).
  - Sudsy and the tool alternate runs; best of 5 wall seconds each, plus the peak resident memory of the best run.
  - The headline is the ratio, which stays stable on a shared runner when absolute times move.
- **Where it runs:** a `speed.yml` workflow on one fixed runner label (`ubuntu-24.04-arm`), started by hand (`workflow_dispatch`) and before each release.
  - It writes `bench/speed.json`: one record per language, task and tool, with times, memories, the runner label, tool versions and the Sudsy commit.
  - The lead commits that file.
- **Generated, not typed:** `cargo xtask codegen` writes the README "Performance" table from `bench/speed.json`; `codegen --check` fails when the table is stale. docs/performance.md keeps only the method and links to the table. docs/ai/performance/numbers.md drops its tool-comparison sections and points to `bench/speed.json`.

## README table
One row per language, task and tool. Language and Task appear only on the first row of their group.

| Language | Task | Compared with | Sudsy speed | Time (Sudsy / tool) | Peak memory (Sudsy / tool) |
|---|---|---|---|---|---|
| JS/TS | Format | Prettier | 19× faster | 0.22 s / 4.21 s | 197 / 298 MB |
| | | Biome | 1.5× slower | 0.22 s / 0.15 s | 197 / 102 MB |

- **Sudsy speed** is always written with Sudsy as the subject: `N× faster than` or `N× slower`, with `same` within 10%.
- **One line under the table:** the runner label, best of 5, the corpus per language (linked), and how to reproduce one row: `scripts/speed.sh <tool>`.

## Phases

### Phase 1: the shared script and workflow
- Write `scripts/speed.sh` and the `speed` key format.
- Port the five existing comparisons (Prettier, Biome, ruff, oxlint, CSharpier, dprint) onto `speed` keys. oxlint and dprint have no parity entry: they get a speed-only entry in `parity_tools.json`, with no `command` and no `coverage`.
- Delete the four old `tests/conformance/speed*.sh` scripts and `speed-tools/` once the new script reproduces their rows. Keep the pinned versions in the entries.
- Add `speed.yml`, `bench/speed.json`, and the codegen table in README.md.
- Update docs/performance.md and docs/development/parity.md (the `speed` key).

**Checkpoint 1:** report the generated table for the ported tools and one `speed.yml` run, before adding tools.

### Phase 2: every language
- Add a `speed` key for every tool in `parity_tools.json` that has a corpus, format and lint:
  - google-java-format, checkstyle;
  - ktlint (format and lint);
  - php-cs-fixer, phpstan;
  - rubocop;
  - stylelint;
  - shfmt, shellcheck;
  - sqlfluff;
  - taplo;
  - terraform, tflint;
  - hadolint;
  - actionlint;
  - markdownlint;
  - yamllint;
  - buf;
  - checkmake;
  - html-validate, htmlhint;
  - eslint and typescript-eslint (eslint's real CLI, not oxlint);
  - the Vue and Svelte plugins;
  - PSScriptAnalyzer and Invoke-Formatter;
  - StyleCop and Roslynator (as `dotnet build` analyzers, if the corpus builds; otherwise not measured, with the reason).
- Where a mirroring preset is missing for lint, that row says `not measured: no mirroring preset`, and the preset goes on the roadmap.
- Run `speed.yml`, commit `bench/speed.json`, and check that the table is generated.

## Done when
- Every tool in `parity_tools.json` has a `speed` key, or a written reason it has none.
- The README table is generated from one `speed.yml` run.
- The old speed scripts are gone.
- Docs point to the table instead of repeating numbers.
