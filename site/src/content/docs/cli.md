---
title: Command line
description: The sudsy commands, exit codes and machine-readable output, for scripts and coding agents.
---

## Commands

| Command | What it does | Exit |
|---|---|---|
| `sudsy check [paths]` | `format --check` and `lint` together, for CI; an unformatted file is an error diagnostic `format` | 1 on an error or an unformatted file |
| `sudsy fix [paths]` | Apply fixes, then format (`--unsafe` also applies unsafe fixes) | 1 if errors remain |
| `sudsy lint [paths]` | Report rule violations | 1 on an error |
| `sudsy format [--check] [paths]` | Format files in place | 1 if `--check` finds changes |
| `sudsy rules [term] [--language <lang>]` | List rules; an exact rule id, or an id of another tool such as `SA1005`, explains the rule | 2 when nothing matches |
| `sudsy config print [file]` | The resolved configuration with the source of each value | 0 |
| `sudsy init [--preset <name>] [--from <tool>] [--agents] [--yes]` | Ask in a terminal, then write `sudsy.yaml`; without a terminal write a starter, migrate another tool's config, or add the agent block | 0 |

Exit 2 means a file could not be processed or the configuration is invalid.

## Machine-readable output

`check`, `lint` and `fix` take `--format human|json|sarif|github`. With `--format json` stdout is one document:

```json
{
  "version": 1,
  "summary": { "errors": 1, "warnings": 0 },
  "diagnostics": [
    {
      "path": "src/a.json",
      "rule": "no-duplicate-keys",
      "category": "correctness",
      "severity": "error",
      "message": "...",
      "range": { "start": { "line": 3, "column": 5, "offset": 41 }, "end": { "line": 3, "column": 8, "offset": 44 } },
      "related": [],
      "fixes": [{ "title": "...", "kind": "safe", "edits": [{ "start": 41, "end": 44, "text": "" }] }]
    }
  ]
}
```

Lines and columns are 1-based; offsets are bytes. A fix `kind` is `safe`, `suggestion` or `unsafe`; `sudsy fix` applies the safe ones. `--format sarif` writes SARIF 2.1.0 for code scanning, `--format github` writes workflow annotations.

An agent's loop: run `sudsy check --format json`, read `diagnostics`, run `sudsy fix`, run the check again, and edit by hand only what is left.

## Rules as data

`sudsy rules <id>` explains one rule in text. Every rule as JSON (id, languages, category, default level, fix kind, options, upstream equivalents and a before and after example) is [`rules.json`](/sudsy/rules.json) on this site, built from the same data as the [rule pages](/sudsy/rules/). The configuration schema is `schemas/sudsy.schema.json` in the repository.

## For coding agents

`sudsy init --agents` adds a short block to `AGENTS.md` (to `CLAUDE.md` when only that file exists) that tells an agent to run `sudsy check` and `sudsy fix`. It writes nothing without the flag, and running it again leaves the file as it is. This site publishes [`llms.txt`](/sudsy/llms.txt), [`llms-full.txt`](/sudsy/llms-full.txt) and a Markdown copy of every page (add `.md` to the page path).

## In CI and pre-commit

The GitHub Action runs `sudsy check --format github`:

```yaml
- uses: bingsubandit/sudsy@v0
  with:
    version: latest   # or a release; `local` uses the sudsy on PATH
    args: src         # optional
```

pre-commit hooks `sudsy-check` and `sudsy-fix` (`language: system`, so `sudsy` must be on PATH):

```yaml
repos:
  - repo: https://github.com/bingsubandit/sudsy
    rev: v0.1.0
    hooks:
      - id: sudsy-check
```
