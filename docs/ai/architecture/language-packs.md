# Language packs (switched off)

Every release channel (archives, npm, PyPI, Homebrew, Scoop, winget, the VS Code extension, cargo-binstall) ships one binary with every language built in: all tree-sitter grammars are linked in, and the C# worker sits beside it. There is no install step, no skipped language, no exit status for a missing language and no LSP prompt, and `sudsy.yaml` `languages:` entries (`required`, `ignored`) are accepted and change nothing. Nothing reads the cache directory or its `state.json`, so a state file or pack left by an older version hides and replaces nothing (test: `crates/cli/tests/builtin.rs`).

The pack code stays in the tree as a switched-off alternative: a build where the grammars of some languages are shared libraries (packs) installed per user and loaded on first use. A pack parses at the speed of a linked-in grammar (same C code, same optimization level); the language's Rust code stays in the `sudsy` binary and a pack holds only `parser.c` and the scanner. It is compiled and tested in exactly one CI job, `pack-mode` (`scripts/pack-mode.sh`).

## Features
- **`static-grammars`** (cargo feature of `cli`, `lsp`, `languages` and the `lang-*` crates; on by default): the grammars are linked in. `core_packs::language!` in each `lang-*` provider expands to the linked grammar; `core-packs` is a dependency of the `lang-*` crates for that macro and nothing else.
- **`pack-mode`** (cargo feature of `cli`, `lsp`, `languages`; off by default): adds the installed set (`languages::installed`), the `cli` subcommand with list, add, remove and sync (`languages_cmd.rs`), the skip of files of a missing language with exit status 3 (`cli/src/missing.rs`) and the LSP prompt and command (`lsp/src/notifier/install.rs`, `lsp/src/server/install.rs`). Without it `languages::installed::Checker` finds nothing missing, and `core-packs` is not a dependency of `languages` and `lsp`. A real pack build is `--no-default-features --features pack-mode` (the grammar crates are not compiled; `core_packs::language!` loads the pack). `languages` refuses to compile with neither feature.

## Pack
- `crates/pack` is a `cdylib` with one cargo feature per language (the grammar crate behind it) exporting `sudsy_language` (Markdown also `sudsy_language_inline` for its inline grammar). The grammar crate's own `tree_sitter_<lang>` symbol is not exported: a Rust cdylib would define it twice.
- File name: `sudsy-<version>-<language>-<target triple>.<dll|so|dylib>`. Packs are locked to the Sudsy version by the name.
- `scripts/pack.sh <out> [language...]` builds them for this machine and writes `<out>/manifest.txt`. The pack-mode CLI is built with `SUDSY_PACK_MANIFEST` pointing at that manifest, so the hashes in the binary are those of the packs. No release builds or uploads packs.

## Manifest and checks (`core-packs`)
- The binary embeds `manifest.txt` (`language file sha256 size` per line; `$SUDSY_PACK_MANIFEST` at build time, empty otherwise, so a build without one lists no pack).
- Load order: read the file, compare its SHA-256 with the manifest (mismatch: refused, the file is not loaded), `dlopen`, resolve the symbol, compare `Language::abi_version()` with the runtime's `MIN_COMPATIBLE_LANGUAGE_VERSION..=LANGUAGE_VERSION`. The library is kept loaded for the process. Hash and `dlopen` read the file separately, so someone who can write the cache directory could swap it in between; they can also replace the binary.
- Install verifies the hash before the file reaches the cache directory (`<name>.part`, then renamed).

## Installed set (pack mode)
- Directory: `$SUDSY_CACHE_DIR`, else `<platform cache dir>/sudsy` (`dirs`): the packs and `state.json` (`{"removed": [language...], "no_prompt": [language...]}`).
- A language is usable when it is not in `removed` and, with `static-grammars` off, its pack loads; a language with no pack in the manifest is in the binary and needs none. Remove adds it to `removed` and deletes its pack; add clears the flag and, if the pack does not load, installs it. With `static-grammars` on, add and remove only flip the flag.
- Download: `curl` on `https://github.com/bingsubandit/sudsy/releases/download/v<version>/<file>` (`core_packs::RELEASE_URL`, the one place); the pack-mode add is the only caller.

## Missing language (pack mode)
`format`, `lint`, `fix` and `check` skip the files of a known language that is not usable, print one stderr line per language and exit 3 when nothing worse happened. A language listed `ignored` under `languages:` is skipped silently with the normal status ([config/sudsy-yaml.md](../config/sudsy-yaml.md)). The LSP keeps the document anyway (invisible to diagnostics, formatting and actions) and sends a `window/showMessageRequest` at every open of a file of that language (at most one unanswered per language) with the actions `Install` and `Don't ask again`. `Install` runs the install code (a download blocks the message loop meanwhile), then serves the waiting documents and asks the client to refresh diagnostics. `Don't ask again` records the language in `no_prompt` of `state.json`. The executeCommand `sudsy.installMissingLanguages` installs every missing, not ignored language of the open documents, or the one named in its argument; the server advertises it only in pack mode, and the VS Code extension has no palette entry for it.
