<p align="center"><img src="assets/logo/soap_3d.png" alt="Sudsy" width="96"></p>

<h1 align="center">Sudsy</h1>

<p align="center"><b>One formatter and linter for your whole repo.</b><br>
One binary and one config file in place of Prettier, ESLint, ruff, buf, terraform fmt and the rest, measured file by file against each of them.<br>
<b>Ready for 6 languages, early support for 20 more.</b></p>

<p align="center">
<a href="https://github.com/bingsubandit/sudsy/releases"><img alt="release" src="https://img.shields.io/github/v/release/bingsubandit/sudsy"></a>
<a href="https://www.npmjs.com/package/@sudsylint/sudsy"><img alt="npm" src="https://img.shields.io/npm/v/@sudsylint/sudsy"></a>
<a href="https://pypi.org/project/sudsy/"><img alt="PyPI" src="https://img.shields.io/pypi/v/sudsy"></a>
<a href="LICENSE"><img alt="MIT" src="https://img.shields.io/badge/license-MIT-blue"></a>
</p>

> **Early release (v0.0.x).** Ready: JavaScript/TypeScript, Python, Protobuf, Terraform, GraphQL and TOML. The rest are partial; see the [status table](#languages). Config may change in minor versions before 1.0.

## Try it

This is read-only: `check` reports and changes no files. It needs no install and no config.

```sh
npx @sudsylint/sudsy check src/      # or: uvx sudsy check src/
```

```
a.py:1:1: error[format]: the file is not formatted; run `sudsy format`
a.py:1:8: warning[unused-import]: `os` imported but unused
a.py:3:6: warning[none-comparison]: compare to None with `is` or `is not`, not `==` or `!=`
b.js:3:18: warning[no-debugger]: Unexpected 'debugger' statement.
```

With no config, lint reports likely bugs only, never style. `sudsy format` rewrites the files. `sudsy fix` applies safe fixes, then formats.

Some languages are small downloads, fetched once with `sudsy languages add` (for example `sudsy languages add toml protobuf`). Until then, their files are skipped with a warning. `sudsy languages list` shows what is installed. The `sudsy-full` release archives contain every language.

## Why

A typical repo runs one tool per language. Each tool has its own install, config file, version pins, editor plugin and CI step. Sudsy is one of each:

```diff
- npx prettier --check . && npx eslint . && ruff format --check && ruff check \
-   && shfmt -d . && buf format -d && terraform fmt -check && markdownlint '**/*.md' \
-   && yamllint . && hadolint Dockerfile
+ sudsy check
```

- **Keep your current style.** A preset reproduces each tool's output. The [table below](#languages) shows how closely, measured on thousands of real-world files per language. Presets are plain config, and every style choice is a named option.
- **Formatting never changes meaning.** Tests and corpus runs check that every formatted file has the same syntax tree as the input, and that formatting twice changes nothing.
- **Your own rules.** Write a rule as a YAML [pattern](docs/rules.md#pattern-rules) (ast-grep style, with fixes), or as a sandboxed [Wasm plugin](docs/rules.md#wasm-plugins).
- **Editor and CI built in.** It includes a language server (with a VS Code extension) and outputs GitHub annotations and SARIF.

**Who it is for:** repos with several languages, where one tool and one config replace five or ten. It fits best today when most of the code is in the ready languages, for example a TypeScript and Python service with Protobuf APIs and Terraform infrastructure. **Who it is not for yet:** a JavaScript-only or Python-only repo already on Biome or ruff. Those tools are faster, and Sudsy adds nothing there except the same config across languages.

## Languages

Each percentage is measured on real-world code against the reference tool's own output, with the tool's default settings.
- **Identical:** the share of files that Sudsy formats byte-for-byte like the reference tool. The main differences per language are in [docs/languages.md](docs/languages.md).
- **Findings:** the share of the reference tool's findings that Sudsy also reports, with the preset named in the table. This is recall only; false positives are not counted here.
- **Preset:** what to put in `extends:` to get that tool's style and rules.

**Ready to format.** At least 85% identical. Lint is ready only where Findings is 85% or more.

| Language | Formats like | Identical | Stable | Lints like | Findings | Presets |
|---|---|---:|---|---|---:|---|
| JavaScript / TypeScript | Prettier | 93% of 2,456 | yes | ESLint, typescript-eslint recommended¹ | 100% | `js/eslint-recommended`, `js/typescript-eslint-recommended` |
| Python | ruff format (Black) | 93% of 1,085 | yes | ruff default rules | 96% | `python/ruff` |
| Protobuf | buf format | 99% of 323 | yes | buf STANDARD | 86% | `protobuf/buf-standard` |
| Terraform (HCL) | terraform fmt | 98% of 318 | yes | tflint recommended | 20% | `hcl/tflint-recommended` |
| GraphQL | Prettier | 98% of 132 | yes | — | — | — |
| TOML | taplo | 95% of 213 | not measured | — | — | `toml/taplo-compat` |

Stable: formatting a second time changes no file in the corpus. ¹ Core rules only. ESLint plugins such as react, import and jsx-a11y, and type-aware TypeScript rules, are not covered yet.

**Partial.** Works, but expect more differences from your current tool. HTML and PowerShell format well but miss the lint bar.

| Language | Formats like | Identical | Lints like | Findings | Preset |
|---|---|---:|---|---:|---|
| HTML | Prettier | 93% of 233 | html-validate | 64% | `html/html-validate-recommended` |
| PowerShell | Invoke-Formatter | 93% of 211 | PSScriptAnalyzer | 34% | `powershell/psscriptanalyzer` |
| PHP | PHP-CS-Fixer (PER-CS) | 88% of 10,382 | PHPStan | 0% | `php/phpstan` |
| Markdown | Prettier | 86% of 64 | markdownlint | 83% | `markdown/prettier-compat`, `markdown/markdownlint` |
| Java | google-java-format | 82% of 5,337 | Checkstyle (Google) | 26% | `java/google` |
| Shell | shfmt | 82% of 82 | ShellCheck | 47% | `shell/shellcheck` |
| SQL | sqlfluff | 78% of 867 | sqlfluff | 4% | `sql/sqlfluff` |
| C# (needs .NET 10) | CSharpier | 72% of 7,615² | StyleCop | not measured | `csharp/csharpier-compat`, `csharp/stylecop` |
| Kotlin | ktlint | 76% of 5,776 | ktlint | 13% | `kotlin/ktlint` |
| Vue / Svelte | Prettier | 83% / 89% | eslint-plugin-vue / -svelte | 87% / 100% | `html/eslint-plugin-vue-recommended`, `…-svelte-recommended` |
| YAML, JSON, CSS, XML | Prettier / Biome / CSharpier | not measured | yamllint / stylelint | 74% / 13% | `yaml/yamllint`, `css/stylelint-standard` |

² The 7,659-file corpus minus the files CSharpier itself leaves alone.

**Lint first:** Ruby is linted, with RuboCop's Lint department at 96% (`ruby/rubocop`), but not reformatted. Dockerfile (hadolint 28%), Makefile (checkmake 74%) and GitHub Actions (actionlint 5%) are formatted too, but no reference formatter has been compared yet.

**Experimental:** C/C++ (not formatted, no rules yet), INI, EditorConfig, .gitignore.

All languages: <!-- generated:languages -->C#, C/C++, CSS, Dockerfile, EditorConfig, Gitignore, GraphQL, HCL, HTML, INI, JSON, Java, JavaScript/TypeScript, Kotlin, Makefile, Markdown, PHP, PowerShell, Protobuf, Python, Ruby, SQL, Shell, TOML, XML and YAML<!-- /generated -->.

Corpora (for example Django for Python, the .NET runtime, Roslyn and ASP.NET Core for C#), tool versions and how to rerun each comparison: [docs/languages.md](docs/languages.md).

## Performance

<!-- generated:speed -->
| Language | Task | Compared with | Sudsy speed | Time (Sudsy / tool) | Peak memory (Sudsy / tool) |
|---|---|---|---|---|---|
| JSON | Format | Prettier | 26× faster | 0.009 s / 0.23 s | 43 / 85 MB |
|  |  | Biome | 4.1× faster | 0.009 s / 0.037 s | 43 / 42 MB |
|  |  | dprint | 1.7× faster | 0.007 s / 0.012 s | 73 / 73 MB |
|  | Lint | eslint-plugin-jsonc | not measured: no mirroring preset | - | - |
| YAML | Format | Prettier | 43× faster | 0.15 s / 6.48 s | 103 / 237 MB |
|  |  | dprint | 1.4× faster | 0.17 s / 0.23 s | 99 / 73 MB |
|  | Lint | yamllint | 18× faster | 0.071 s / 1.25 s | 62 / 55 MB |
| JS/TS | Format | Prettier | 23× faster | 0.21 s / 4.87 s | 172 / 298 MB |
|  |  | Biome | 1.5× slower | 0.27 s / 0.18 s | 158 / 104 MB |
|  | Lint | ESLint | 16× faster | 0.27 s / 4.30 s | 189 / 451 MB |
|  |  | typescript-eslint | 24× faster | 0.32 s / 7.54 s | 240 / 730 MB |
|  |  | Biome | 1.4× faster | 0.27 s / 0.38 s | 205 / 146 MB |
|  |  | oxlint | 4.2× slower | 0.24 s / 0.057 s | 199 / 160 MB |
| CSS | Format | Prettier | 39× faster | 0.14 s / 5.61 s | 157 / 392 MB |
|  | Lint | stylelint | 7.5× faster | 0.27 s / 2.04 s | 113 / 355 MB |
| HTML | Format | Prettier | 54× faster | 0.026 s / 1.40 s | 98 / 169 MB |
|  | Lint | html-validate | 25× faster | 0.090 s / 2.27 s | 104 / 285 MB |
|  |  | HTMLHint | 4.0× faster | 0.066 s / 0.26 s | 82 / 100 MB |
|  |  | html-eslint | 2.0× faster | 0.062 s / 0.12 s | 82 / 75 MB |
| Angular | Format | Prettier | not measured: Prettier picks its Angular parser from the .component.html name, which the corpus files do not carry | - | - |
| TOML | Format | taplo | 1.6× faster | 0.012 s / 0.019 s | 45 / 40 MB |
| Python | Format | ruff | 4.8× slower | 0.31 s / 0.066 s | 224 / 58 MB |
|  | Lint | ruff | 2.1× slower | 0.34 s / 0.17 s | 245 / 57 MB |
| SQL | Format | sqlfluff | 386× faster | 0.012 s / 4.63 s | 57 / 52 MB |
|  | Lint | sqlfluff | 122× faster | 0.043 s / 5.27 s | 52 / 52 MB |
| Shell | Format | shfmt | 2.6× slower | 0.059 s / 0.023 s | 79 / 52 MB |
|  | Lint | ShellCheck | 1913× faster | 0.089 s / 170.25 s | 86 / 9735 MB |
| Java | Format | google-java-format | 19× faster | 1.09 s / 20.53 s | 646 / 998 MB |
|  | Lint | Checkstyle | 153× faster | 0.96 s / 146.16 s | 591 / 1173 MB |
| Kotlin | Format | ktlint | 30× faster | 1.61 s / 48.42 s | 501 / 715 MB |
|  | Lint | ktlint | 42× faster | 1.14 s / 48.01 s | 485 / 683 MB |
| PHP | Format | PHP-CS-Fixer | 161× faster | 1.77 s / 285.46 s | 1161 / 70 MB |
|  | Lint | PHPStan | not measured: the analysis runs per project folder, not on a folder of unrelated files | - | - |
| PowerShell | Format | Invoke-Formatter | not measured: no PowerShell (pwsh) on the host that wrote this table | - | - |
|  | Lint | PSScriptAnalyzer | not measured: no PowerShell (pwsh) on the host that wrote this table | - | - |
| Protobuf | Format | buf | 3.6× faster | 0.069 s / 0.25 s | 118 / 74 MB |
|  | Lint | buf | not measured: the analysis runs per module, not on a folder of unrelated files | - | - |
| HCL | Format | terraform fmt | 3.5× faster | 0.11 s / 0.40 s | 123 / 87 MB |
|  | Lint | tflint | 23× faster | 0.20 s / 4.69 s | 120 / 89 MB |
| C# | Format | CSharpier | 3.3× faster | 4.36 s / 14.38 s | 572 / 456 MB |
|  | Lint | StyleCop | not measured: the compiler skips analyzers when the compilation has errors, and the C# corpus does not compile on its own | - | - |
| XML | Format | CSharpier | 4.0× faster | 0.11 s / 0.44 s | 104 / 97 MB |
| C# (.editorconfig) | Format | dotnet format | not measured: its fixtures are small .editorconfig cases, each in a folder of its own, not a corpus | - | - |
| Ruby | Lint | RuboCop | 88× faster | 3.00 s / 264.55 s | 1180 / 377 MB |
| Vue | Lint | eslint-plugin-vue | not measured: no runner yet: the flat config for .vue files needs vue-eslint-parser and a corpus of .vue files | - | - |
| Svelte | Lint | eslint-plugin-svelte | not measured: no runner yet: the flat config for .svelte files needs svelte-eslint-parser and a corpus of .svelte files | - | - |
| SCSS | Lint | stylelint-scss | not measured: no mirroring preset | - | - |
| Less | Lint | stylelint-less | not measured: no mirroring preset | - | - |
| Markdown | Lint | markdownlint | 7.5× faster | 0.096 s / 0.72 s | 87 / 198 MB |
| Dockerfile | Lint | hadolint | 36× faster | 0.071 s / 2.57 s | 70 / 74 MB |
| GitHub Actions | Lint | actionlint | 2.1× slower | 0.14 s / 0.065 s | 76 / 71 MB |
| Makefile | Lint | checkmake | 3.7× slower | 0.059 s / 0.016 s | 70 / 28 MB |

Runner: Linux aarch64, 8 cores. Best of 5 runs, a new process each, caches off, `format --check` and `lint` with the Sudsy preset that mirrors the tool. Corpus ([docs/languages.md](docs/languages.md)): JSON: the valid files of JSONTestSuite; YAML: the GitHub Actions workflows of real repositories, 8 copies, without 16 files Sudsy cannot parse and the GitHub Actions workflows of real repositories and the yaml-test-suite cases; JS/TS: three.js src, express lib and test, 8 copies and TypeScript sources of vuejs/core, docusaurus, excalidraw and MUI; CSS: the CSS, SCSS and Less corpus and the CSS corpus (css-tricks, Bootstrap, and others); HTML: the HTML corpus (mdn learning-area and others), without 37 files Sudsy cannot parse and the HTML corpus (Prettier tests, mdn learning-area); TOML: the toml-test valid files and the TOML fixtures of the repository; Python: the Python corpus (Django, Requests, Flask, Click, HTTPX, pytest, Black); SQL: the ANSI dialect fixtures of sqlfluff, without 3 files Sudsy cannot parse; Shell: the shell script corpus; Java: the Java corpus (5 projects); Kotlin: the Kotlin corpus; PHP: the PHP corpus; Protobuf: the Protobuf corpus; HCL: the Terraform corpus; C#: dotnet/runtime CoreLib, without 14 files Sudsy cannot parse; XML: the XML fixtures and the project files of dotnet/aspnetcore, without 1 files Sudsy cannot parse; Ruby: the Ruby corpus; Markdown: the dotnet/runtime documentation; Dockerfile: the Dockerfile corpus; GitHub Actions: the GitHub Actions workflows of real repositories; Makefile: the Makefile corpus. Rerun one row: `scripts/speed.sh <tool>`.
<!-- /generated -->

Sudsy does the same work as each tool: `format --check` and `lint` with the preset that mirrors it. The binary is about 53 MB: it holds the parsers for every language and a Wasm runtime for plugins. Method: [docs/performance.md](docs/performance.md).

## Install

| Channel | Command |
|---|---|
| npm | `npm i -D @sudsylint/sudsy` |
| PyPI | `pip install sudsy` or `uvx sudsy` |
| Homebrew | `brew install bingsubandit/sudsy/sudsy` |
| Scoop | `scoop bucket add sudsy https://github.com/bingsubandit/scoop-sudsy; scoop install sudsy` |
| Binary | [GitHub releases](https://github.com/bingsubandit/sudsy/releases): Linux, macOS and Windows, each for x64 and arm64 |

Only C# needs more. It uses Roslyn in a worker process that every channel above bundles, and the worker needs the [.NET 10 runtime](https://dotnet.microsoft.com/download/dotnet/10.0). Every other language works without .NET.

## Configure

```sh
sudsy init --preset python/ruff     # a starter sudsy.yaml with one or more presets
sudsy init --from prettier          # read your .prettierrc and write the matching options
```

`--from` reads the config of Prettier, Biome, ESLint, stylelint, yamllint, CSharpier, StyleCop, html-validate, htmlhint and html-eslint. For ruff, use the `python/ruff` preset and set its options by hand.

```yaml
# sudsy.yaml
extends: [preset:js/eslint-recommended, preset:python/ruff, preset:java/google]
exclude: ["**/generated/**"]
python:
  format: { line_width: 100 }
overrides:
  - files: ["tests/**"]
    python: { lint: { rules: { unused-import: "off" } } }
```

Existing `.editorconfig` keys apply. `sudsy config print` shows every resolved value and where it came from. `sudsy rules <id>` explains a rule. Reference: [docs/configuration.md](docs/configuration.md).

## CI and editors

```sh
sudsy check --format github                # annotations in GitHub Actions
sudsy check --format sarif > sudsy.sarif   # GitHub code scanning
```

`check` exits 1 when a file is unformatted or a rule reports an error. `sudsy lsp` is a language server over stdio, with diagnostics, quick fixes and formatting. VS Code: install the `.vsix` for your platform from the [release](https://github.com/bingsubandit/sudsy/releases) with `code --install-extension sudsy-<platform>-<version>.vsix`. See [docs/editors-and-ci.md](docs/editors-and-ci.md).

## More

- [Languages](docs/languages.md): status, options and presets per language
- [Configuration](docs/configuration.md): `sudsy.yaml`, presets, `.editorconfig`, the CLI
- [Rules](docs/rules.md): built-in rules, pattern rules, Wasm plugins
- [Editors and CI](docs/editors-and-ci.md) and [performance](docs/performance.md)
- [Roadmap](docs/ai/roadmap.md), [changelog](CHANGELOG.md), [contributing](CONTRIBUTING.md)

MIT licensed. Test inputs from other projects: [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).
