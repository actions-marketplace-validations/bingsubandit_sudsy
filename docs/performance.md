# Performance

Sudsy is in the same speed range as the Rust tools. It is not faster than ruff, Biome or oxlint on their own languages. It is much faster than Prettier and CSharpier.

## Method

- Check mode (`format --check`, `lint`). A new process per run. Caches off.
- Best of 5 runs (Python, JavaScript). Best of 3 (JSON, YAML). Wall-clock seconds.
- Peak memory is the maximum resident set of the best run.
- Lint compares each tool's default rules. Rule counts differ: ruff has 407 rules, Sudsy has far fewer.
- Run-to-run noise is 10-15% on the Windows machine. Compare ratios, not absolute values.

## Python and JavaScript

Machine: Linux aarch64, 8 cores (Neoverse-V2), 15 GB. Release builds. ruff 0.17.4, Biome 2.6.4, oxlint 1.94.0, Prettier 3.9.10.

- Python: Django 5.2.8, 883 files, 8 MB. ruff runs with `--isolated --no-cache`.
- JavaScript: three.js `src` plus express `lib` and `test`, 8 copies, 848 files, 7 MB.

| Task | Tool | Time | Time vs Sudsy | Peak memory | Memory vs Sudsy |
|---|---|---:|---|---:|---|
| Format Python | Sudsy | 0.18 s | 1x | 180 MB | 1x |
| Format Python | ruff | 0.04 s | 4.5x faster | 47 MB | 3.8x less |
| Lint Python | Sudsy | 0.11 s | 1x | 171 MB | 1x |
| Lint Python | ruff | 0.11 s | same | 47 MB | 3.6x less |
| Format JS | Sudsy | 0.22 s | 1x | 197 MB | 1x |
| Format JS | Biome | 0.15 s | 1.5x faster | 102 MB | 1.9x less |
| Format JS | Prettier | 4.21 s | 19x slower | 298 MB | 1.5x more |
| Lint JS | Sudsy | 0.11 s | 1x | 206 MB | 1x |
| Lint JS | oxlint | 0.05 s | 2.2x faster | 164 MB | 1.3x less |
| Lint JS | Biome | 0.32 s | 2.9x slower | 146 MB | 1.4x less |

**Lint compares default rules.** With no config, Sudsy runs only its bug-only set. The other tools run their own default sets, and the sets differ (ruff has 407 rules; Sudsy has far fewer). A lint run with a preset that mirrors the other tool does more work. That run is not measured here.

Where Sudsy is slower (ratios from the table):

- Python format: ruff is 4.5 times faster.
- JS format: Biome is 1.5 times faster.
- JS lint: oxlint is 2.2 times faster.
- Memory: Sudsy uses 2 to 4 times the memory of ruff on Django.

Where it is faster: JS format is about 19 times faster than Prettier. JS lint is about 3 times faster than Biome.

## JSON and YAML

These runs are on a different machine from the table above: Windows 11, AMD Ryzen 5 3600 (6 cores, 12 threads), 32 GB. Release build. Best of 3. Noise is 10-15%, so ratios under about 1.2x are within noise. Prettier 3.9.10, Biome 2.6.4, dprint 0.64.3, Node 24.22.0. Biome's YAML support is not stable and is excluded.

| Input | Sudsy | Prettier | Biome | dprint |
|---|---:|---:|---:|---:|
| JSON, 95 small files | 0.10 s | 0.67 s (6.7x slower) | 0.36 s (3.6x slower) | 0.37 s (3.7x slower) |
| JSON, one 11.1 MB file | 2.03 s | 9.19 s (4.5x slower) | 1.45 s (1.4x faster) | 1.91 s (1.1x faster) |
| YAML, 308 small files | 0.11 s | 1.13 s (10x slower) | n/a | 0.35 s (3.2x slower) |
| YAML, one 5.0 MB file | 2.39 s | 20.19 s (8.4x slower) | n/a | 3.06 s (1.3x slower) |

Biome is faster than Sudsy on the one large JSON file. dprint is about equal on it.

## C#

C# formatting uses Roslyn in a worker process. Machine: 12 cores (model not stated in the source). The machine ran about 15% slower on the day of this run.

| Input | CSharpier 1.3.0 | Sudsy, cold start | Sudsy, warm worker |
|---|---:|---:|---:|
| .NET CoreLib, 1,679 files, 27.8 MB | 17.3-17.8 s | 3.8-3.9 s | 2.4-2.6 s |
| Full corpus, 7,659 files, 89.8 MB | 42.3-43.3 s | 9.7-10.3 s | 8.0-8.1 s |

Sudsy is 4.3 to 4.5 times faster than CSharpier with a cold worker, and 5.3 to 7.3 times with a warm one. One file at a time, with a warm worker, takes a median of 1.4-1.5 ms.

### C# lint

Lint uses Roslyn's semantic model, one file at a time with one worker.

| Tree | Files | `fast` | `standard` | `deep` | `standard` median / p95 per file |
|---|---:|---:|---:|---:|---|
| CoreLib | 1,679 | 5.1 s | 56.1 s | 57.1 s | 16 / 110 ms |
| Roslyn | 2,858 | 6.1 s | 77.3 s | 82.6 s | 11 / 72 ms |
| aspnetcore | 3,122 | 4.5 s | 74.1 s | 85.4 s | 12 / 42 ms |

`standard` is 10 to 15 times slower than `fast`. This is the slowest part of Sudsy. Use `--analysis fast` for quick runs.

## Other languages

Not compared against other tools on wall-clock time: PHP, Java, Kotlin, Ruby, SQL, HTML, CSS, Markdown, Shell, HCL, Protobuf, TOML, XML. Sudsy tracks instruction counts for these to catch regressions. They are not a comparison with other tools.

## Binary size

The stripped Linux release binary is about 53 MB (52,990,712 bytes, measured with `scripts/binsize.sh`). It holds the parsers of every language and a Wasm runtime (wasmtime, about 20 MB) for plugins. C# also needs the bundled worker directory and the .NET 10 runtime. Size of the current release: not measured.

## Rerun

| What (needs a source checkout) | Command |
|---|---|
| JSON and YAML | `tests/conformance/speed-json-yaml.sh` |
| Python and JavaScript | `tests/conformance/speed-lint-format.sh` |
| C# | `tests/conformance/speed.sh` |
| Binary size | `scripts/binsize.sh` |
| Instruction counts of a change | `scripts/bench.sh` |

The scripts need the other tools installed. Details: [ai/performance/numbers.md](ai/performance/numbers.md), [ai/performance/tooling.md](ai/performance/tooling.md).
