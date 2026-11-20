# Editors and CI

## VS Code

The extension is not on the VS Code Marketplace. Install the `.vsix` for your platform from the [GitHub release](https://github.com/bingsubandit/sudsy/releases):

```sh
code --install-extension sudsy-<platform>-<version>.vsix
```

Platforms: `linux-x64`, `linux-arm64`, `darwin-x64`, `darwin-arm64`, `win32-x64`, `win32-arm64`. The package bundles `sudsy` and the C# worker. It starts `sudsy lsp` and adds nothing else. It covers all supported languages.

Enable format on save per language, and fix on save:

```jsonc
// .vscode/settings.json
{
  "[python]": { "editor.defaultFormatter": "sudsy.sudsy" },
  "[javascript]": { "editor.defaultFormatter": "sudsy.sudsy" },
  "editor.formatOnSave": true,
  "editor.codeActionsOnSave": { "source.fixAll.sudsy": "explicit" }
}
```

The extension sets no defaults.

| Setting | Meaning |
|---|---|
| `sudsy.importStrategy` | `fromEnvironment` (default): use `sudsy.serverPath`, else `sudsy` on PATH, else the bundled binary. `useBundled`: always the bundled binary. |
| `sudsy.serverPath` | Path to a `sudsy` binary. User settings only. |
| `sudsy.csharpWorkerPath` | Path to the C# worker. Default: beside the executable. User settings only. |
| `sudsy.trace.server` | Protocol trace in the Sudsy output channel. |

- A project can provide its own binary with `npm i -D @sudsylint/sudsy`. For a `sudsy` in a terminal, install the CLI separately.
- The two path settings cannot be set in a workspace `.vscode/settings.json`. This stops a repository from choosing which program runs.
- The extension does not run in Restricted Mode. It starts a program and reads repository config, including plugins.
- It validates `sudsy.yaml` against the JSON Schema. Install `redhat.vscode-yaml` to see the hints.
- Palette commands: restart server, show output, "Sudsy: Install Missing Languages" (fetches the packs of the open files).
- Every diagnostic has the quick fix "Show documentation for <rule>".

## Other editors

`sudsy lsp` is a language server over stdio. Point any LSP client at it.

```sh
sudsy lsp
```

What it supports:

- Diagnostics by pull (`textDocument/diagnostic`). A client without pull support gets none.
- Quick fixes for each fix. Unsafe fixes are marked "(unsafe)".
- "Fix all `<rule>` problems in this file" and `source.fixAll.sudsy` for fix on save.
- Whole-document formatting. Range formatting is not offered.
- Config reload when `sudsy.yaml` or `.editorconfig` changes.
- Only `file:` URIs. Positions are UTF-16.

The server uses the config of the first workspace folder. For C#, a missing .NET 10 runtime shows one message per session. Other languages keep working.

## GitHub Actions

```yaml
# .github/workflows/sudsy.yml
name: sudsy
on: [push, pull_request]
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: 22 }
      - run: npx @sudsylint/sudsy check --format github
```

`--format github` prints one annotation per finding. The step fails on any error or unformatted file. If the repo has C# files, add `actions/setup-dotnet` with `dotnet-version: 10.0.x`.

### Language packs in CI

Files of a language whose pack is missing are skipped with a warning, and the exit code is 3 (when nothing else failed). Fetch the packs first. Add this step before `check`:

```yaml
      - run: npx @sudsylint/sudsy languages add hcl protobuf shell toml markdown
```

List the languages your repo uses. Or use a `sudsy-full-<version>-<platform>` archive from the GitHub release, which contains every language. More: [language packs](configuration.md#language-packs).

### Code scanning (SARIF)

```yaml
      - run: npx @sudsylint/sudsy check --format sarif > sudsy.sarif
        continue-on-error: true
      - uses: github/codeql-action/upload-sarif@v3
        with: { sarif_file: sudsy.sarif }
```

SARIF is version 2.1.0. It lists results and the rule table. It does not carry fixes.

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Success. |
| 1 | An unformatted file or a lint error. |
| 2 | A file could not be processed, or the config is invalid. |
| 3 | Files of a language whose pack is missing were skipped. |

## Other output

`--format json` gives `{version, summary, diagnostics}` with byte offsets, line and column ranges and fix edits. Use it in scripts. The [site](https://bingsubandit.github.io/sudsy/cli/) documents the shape, and publishes `llms.txt` and a Markdown copy of every page for coding agents.

## Coding agents

`sudsy init --agents` adds a short block to `AGENTS.md` (or to `CLAUDE.md` when only that exists) that tells an agent to run `sudsy check --format json` and `sudsy fix`. It writes nothing without the flag and a second run changes nothing.

## pre-commit

Not documented. No pre-commit hook definition exists in the repository. Call `sudsy` from a local hook yourself:

```yaml
# .pre-commit-config.yaml
repos:
  - repo: local
    hooks:
      - id: sudsy
        name: sudsy
        entry: sudsy check
        language: system
        pass_filenames: true
```

This example is not tested.
