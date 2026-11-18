# Sudsy documentation

Sudsy is a formatter and linter for 26 languages in one binary. Start with the [project README](../README.md) for install and a quick try.

| Page | Read it to |
|---|---|
| [Languages](languages.md) | See each language's status, how closely it matches its reference tool, and its options. |
| [Configuration](configuration.md) | Write `sudsy.yaml`, pick presets, suppress rules, use the CLI. |
| [Rules](rules.md) | List and explain rules. Write pattern rules and Wasm plugins. |
| [Performance](performance.md) | See measured speed, memory and binary size against other tools. |
| [Editors and CI](editors-and-ci.md) | Set up VS Code, other editors, GitHub Actions and SARIF. |

## Quick start

```sh
npx @sudsylint/sudsy check .          # report only, changes nothing
sudsy init --preset python/ruff       # write a starter sudsy.yaml
sudsy format .                        # rewrite files
sudsy fix .                           # apply safe fixes, then format
sudsy rules <id>                      # explain a rule
sudsy config print <file>             # show resolved config and its sources
```

## Also in this repository

- [Roadmap](ai/roadmap.md)
- [Changelog](../CHANGELOG.md)
- [Contributing](../CONTRIBUTING.md)

Internals and design notes for contributors and AI agents: [docs/ai/](ai/README.md).
