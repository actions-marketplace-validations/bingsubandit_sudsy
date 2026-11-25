# Rule texts

Every rule has two texts: `description` ("What it does" on the rule page) and `why` ("Why it matters"). A rule that has no `why` is listed in `docs/ai/rules/undescribed.txt`; the list only shrinks (`cargo xtask codegen` drops a rule once it has its text and fails for a new rule without one), and `cargo xtask new-rule` writes both keys. Plan and order of the batches: [../design/site-refresh.md](../design/site-refresh.md).

Where they live: the `description` and `why` keys of a pattern or family rule file (a `>-` block: lines fold into one, a blank line starts a paragraph), the `description:` and `why:` of `rule!` (a string; `\n` between paragraphs) or the fields of `RuleMeta`. The same text reaches `rules.json` (paragraphs as Markdown), the rule page, `sudsy rules <id>` and the Markdown twins. Inline code in backticks works in both. The editor message, the SARIF table, the JSON schema and the list of `sudsy rules` show the first sentence of `description` (the whole text for a rule that has no `why` yet), so it stands alone.

A rule id that several languages implement with one reason (regex and variable rules) has its `why` once, as a section `== <id>` of `crates/core-rules/texts/why.txt`. A language rule without its own `why` takes it; a language writes its own only where its facts differ. The `description` stays per language (API names differ). `cargo xtask codegen` refuses the same `why` under one id in two providers with different descriptions. Texts cost nothing while linting: a `>-` block of a rule file is not folded at load; `RuleMeta::source` holds the file text and `description_text()`, `own_why()`, `why_text()` fold it when a page, `sudsy rules` or `rules.json` asks.

## Writing
- **Facts come from the code.** Read the rule's YAML or Rust, its options and its fixtures, then what the upstream tool documents for the rule (read only, our own words, no ids of other tools in the text; the upstream table lists them). Run the CLI on a small input for every claim about what is and is not reported, and keep the inputs in `.cargo/tmp/`. Where the text and the rule disagree the rule is checked: a wrong rule is fixed with a test, or listed in [../bugs.md](../bugs.md) with the input; the text never goes vague to hide it.
- **`description`**: the first sentence says what is reported ("Reports a ..."), by itself. Then what is not reported, the forms the rule accepts, and the options that change it, in plain words: complete enough to decide a case without reading the code.
- **`why`**: one to four sentences, specific to this rule: what breaks, when, for whom. A text that would fit another rule is rejected ("improves readability", "can cause problems"). A style rule names the convention and what it buys; it does not invent a danger. State a trade-off when there is one.
- **How to fix** is the last paragraph of `why`, only where the fix is not obvious from the rule (a third key would need a place in `RuleMeta`, the macro, the JSON, the page and the CLI for a sentence that belongs to the same argument). The fixed example on the rule page shows the rest.
- **Verify what you cannot run.** A claim about another tool's behaviour (an error message, a default) comes from its documentation or is left out; name it in the report as unverified.

## Three examples
A rule that prevents a harm (`delete-apt-lists`):

> **description.** Reports a `RUN` that updates the apt package lists and does not delete them. It runs `apt-get update` or `apt update` and has no `rm` of `/var/lib/apt/lists` after it in the same `RUN`. `apt-get clean` does not count, it empties another directory. A `RUN` with a `--mount` of type `cache` or `tmpfs` on `/var/lib/apt/lists` is not reported.
>
> **why.** `apt-get update` downloads the package index files into `/var/lib/apt/lists`, often tens of megabytes that no program needs at run time. They are written into the layer of that `RUN`, and deleting them in a later `RUN` hides them but does not make the image smaller, because the earlier layer still holds the files.
>
> To fix, remove them in the same `RUN`: `apt-get update && apt-get install -y ... && rm -rf /var/lib/apt/lists/*`.

What makes it good: the first sentence says the case, the next ones the exact edges (found by running the CLI: `rm` before the update does not count, `apt-get clean` does not count); the harm names the mechanism (layers), not "bloat"; the fix is the last paragraph.

A convention rule (`use-workdir-not-cd`):

> **description.** Reports a `RUN` that calls `cd`, anywhere in its script (also in a subshell such as `(cd sub && make)`).
>
> **why.** `cd` changes the directory for that one `RUN`; the next instruction starts again in the `WORKDIR`, so it is easy to believe you are in a directory you are not in. `WORKDIR` states the directory once, creates it when it is missing, and applies to every later `RUN`, `CMD`, `ENTRYPOINT`, `COPY` and `ADD`.

No invented danger: it says which convention and what it buys.

A rule with exceptions and a trap (`pipefail-shell`):

> **description.** Reports a `RUN` with a pipe (`a | b`) in a stage whose `SHELL` does not set `pipefail`. No earlier `SHELL` instruction of the stage has a word that contains `pipefail` (`SHELL ["/bin/bash", "-o", "pipefail", "-c"]`). A `set -o pipefail` inside the `RUN` itself does not count. A new `FROM` starts the check again.
>
> **why.** The exit status of a pipe is the one of its last command. `curl -f https://example.com/x | tar xz` succeeds even when curl fails and tar gets nothing, so the build goes on with a missing or broken file and the error shows up much later. With `pipefail` any failing command of the pipe fails the `RUN`.
>
> To fix, set the option for the stage with `SHELL ["/bin/bash", "-o", "pipefail", "-c"]`; the default `/bin/sh` of some images (Debian's dash) does not have the option.

The harm is a concrete case (a download piped to `tar`), and the surprising edge (`set -o pipefail` in the script is not seen) is in `description`, where a reader who hit it looks first.

## Checks
- `cargo xtask codegen --check` fails for a rule without `why` that `undescribed.txt` does not list.
- A second agent reads a sample of a batch against the code (15 rules); more than one false statement sends the batch back.
