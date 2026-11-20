# Discoverability: plan

Approved by the user 2026-11-20: phases 1-3 queued, phase 4 waits until JS/TS and Python are Ready.

Goal: people and coding agents who search for a formatter or linter, or for a specific rule of another tool, find Sudsy, and what they find is accurate.

## Decisions (user, 2026-11-20)

- **Domain:** stay on `bingsubandit.github.io/sudsy` for now. `github.io` is on the Public Suffix List, so it ranks as its own site. The cost is small: no brand in the domain, and a subpath. A later custom domain is cheap, because GitHub Pages 301-redirects the github.io URLs to it. Every page sets a canonical URL from `site/config.mjs`, so the switch stays clean.
- **Names:** VS Code publisher `sudsy`, Open VSX namespace `sudsy`, winget id `Sudsy.Sudsy`. All three were free on 2026-11-20. The user registers the publisher and the namespace; phase 1 renames `Coox` in the repo.

## Current state (origin/main 69ad8611)

| Surface | State | Problem |
|---|---|---|
| Website | Astro site on GitHub Pages: getting-started, index, and one rules page (`site/src/content/docs/rules/index.mdx`, all rules from `docs/ai/rules/rules.json`) | One page for every rule, so a search for a rule name lands nowhere specific. No per-language pages and no comparison pages. |
| npm / PyPI / Scoop / winget metadata | `scripts/package-channels.py`: DESC "One formatter and linter for JS/TS, Python, Protobuf, Terraform and more", no `keywords`, no PyPI classifiers | Registry search ranks on keywords and description, and Sudsy has none of the searched words: prettier, eslint, ruff, linter, formatter, hadolint. |
| Package READMEs | `packaging/README.md`, hand-written | **Inaccurate:** it says "Ready for JavaScript/TypeScript, Python, Protobuf, Terraform, GraphQL and TOML", while the generated table shows JS/TS, Python and Terraform In progress. npmjs.com and pypi.org show this text. |
| Names | VS Code publisher `Coox`, winget `Coox.Sudsy` | Old organisation name. |
| VS Code | `.vsix` in releases; Marketplace publish is a manual job (`release.yml` `marketplace`) | No Open VSX, so Cursor, Windsurf and VSCodium don't find it. |
| CI integration | No `action.yml`, no `.pre-commit-hooks.yaml` | The GitHub Marketplace and pre-commit are where people browse for linters. |
| Config schema | `schemas/sudsy.schema.json` | Not in SchemaStore. |
| Agents | No `llms.txt`, no Markdown twin of the docs, nothing in `sudsy init` for agent instructions | Agents read docs as text and follow the repo's `AGENTS.md`/`CLAUDE.md`. |

## How tools like this get found

1. A search for the incumbent's name plus a problem or a rule id ("eslint alternative", "hadolint DL3008"). Sudsy already maps hundreds of other tools' rule ids in `docs/ai/parity/*.yaml`, so it has the data to rank for each of them.
2. Registry search: npm, PyPI, VS Code Marketplace, Open VSX, GitHub Marketplace, pre-commit, SchemaStore.
3. Agents: web search, docs as text (`llms.txt`, Markdown), the repo's instruction file and, over time, training data (stars, links, posts).
4. Word of mouth, which only works once the product holds up on first use.

## Phase 1: accurate and findable metadata (one agent step, no outside action)

Built. How it works: [../development/site.md](../development/site.md) (pages, search basics, the check) and [../development/ci-release.md](../development/ci-release.md#package-managers-githubworkflowspublishyml-packaging-scriptspackage-channelspy) (package text). Outside actions still open for the user: register the VS Code publisher and the Open VSX namespace `sudsy`, claim `Sudsy.Sudsy` on winget with the first submission. The copyright line of `LICENSE` keeps its legal holder.

- **Generated package text.** The package READMEs and descriptions come from the support table, like the README header. The stale "Ready for …" claim goes away and can't come back.
- **Keywords and classifiers.** npm `keywords`, PyPI classifiers and keywords. Tool names are used only where a preset reproduces that tool (prettier, eslint, ruff, stylelint, hadolint, shellcheck, …), plus formatter, linter, code-quality, monorepo.
- **Rename `Coox`:** the VS Code publisher becomes `sudsy`; winget `Coox.Sudsy` becomes `Sudsy.Sudsy` in the files, ids, release workflow and docs.
- **One page per rule:** `/rules/<id>/`, generated from `rules.json`. Each page lists the equivalent ids from the parity catalogs, the languages, a fixture before/after, the options and the measured coverage for that tool. A per-tool index (`/compare/eslint/`) lists every rule of that tool with its Sudsy rule and the coverage. Pages need real content, because thin pages get down-ranked.
- **One page per language:** parity numbers, speed row and preset.
- **Search basics:** sitemap.xml, canonical URLs from `site/config.mjs`, titles that name the tool and the language, an OpenGraph image.
- **Check:** a script fails when a rule or language has no page.

## Phase 2: agents (one agent step)

- `llms.txt` and `llms-full.txt` at the site root, plus a `.md` twin of every page, generated from the same docs.
- Document the machine-readable CLI output there (`check --format json`, `rules --json`; verify both exist).
- `sudsy init --agents`, opt-in: add a short "run `sudsy check`; fix with `sudsy fix`" block to the repo's `AGENTS.md` (or `CLAUDE.md`). Never written without the flag.
- MCP server: deferred. An agent with a shell runs the CLI directly.

## Phase 3: integrations (one agent step, plus user actions)

- GitHub Action (`action.yml`: install, `sudsy check --format github`). The GitHub Marketplace listing is a user action.
- `.pre-commit-hooks.yaml`.
- An Open VSX publish job next to the Marketplace job.
- A SchemaStore pull request for `sudsy.yaml` (user action: a PR to another project).

## Phase 4: outreach (user, timed)

Repo description and topics, "awesome" list entries, a launch post, and a write-up of the parity method. Wait until JS/TS and Python are Ready: a first visit that shows the two biggest languages In progress is hard to undo.

## Measure

Baseline before phase 1:
- 20 fixed search queries, kept in the repo, recording whether and where Sudsy appears;
- npm and PyPI weekly downloads;
- GitHub traffic (repo admin, user).

Repeat monthly. This is a trend signal, not a ratchet: search results vary by engine and region.

## Critical notes

- Accuracy before reach. Everything published is generated from the parity data.
- Keyword use stays truthful: "prettier" is fair because a preset reproduces it at a measured rate; "drop-in replacement" is not, while JS/TS is In progress.
- Outward-facing steps need the user: publishing, registering names, PRs to other projects, posts, repo settings. Agents prepare them and do not run them.

## Cost

Phases 1-3: about one agent step each, mostly generators over existing data (`rules.json`, the parity catalogs, the support table). Phase 4: the user's time.
