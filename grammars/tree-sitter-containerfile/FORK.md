# Fork of tree-sitter-containerfile

Upstream: crates.io `tree-sitter-containerfile` 0.9.2 (wharflab/tree-sitter-containerfile at 1df9124b22a72dde8683aab6f762ff4857acfeab, MIT, [LICENSE](LICENSE)). Regenerate `src/parser.c` after editing `grammar.js` with `scripts/grammar-regen.sh containerfile`; `src/scanner.c` is upstream's but for the `#` rule below.

`bindings/rust` is trimmed to the language constant (no queries or doctest, no wasm branch) so the workspace's fmt and clippy pass; `tree-sitter generate` does not touch it. The upstream queries, README and Cargo.lock are not copied.

Changes to `grammar.js` (valid Dockerfiles the upstream grammar rejected):
- `shell_command`: a fragment after a line continuation may start with `[` (`_shell_continued_fragment`), so a script line such as `[ -s x ]; \` is shell text. Only the first fragment of a command can be a JSON array. 81 of the 83 corpus files for which tree-sitter reported an error (docker-library python, postgres, golang) are now clean.

`src/scanner.c`: a `#` is a comment at column 0 or after a blank, not after other text (`ADD ${A}#${B} /`); `expansion_operator` takes `#`, `##`, `%`, `%%` (`ARG X=${V#linux/amd64}`).
