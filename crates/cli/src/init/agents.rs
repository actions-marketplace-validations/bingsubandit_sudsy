//! `sudsy init --agents`: a short block in the repo's agent instructions (`AGENTS.md`, or
//! `CLAUDE.md` when only that exists) that says how to run Sudsy. Written only on request, once.

use anyhow::Result;
use std::path::Path;

const START: &str = "<!-- sudsy:start -->";
const END: &str = "<!-- sudsy:end -->";

const BLOCK: &str = "## Sudsy

This repo is formatted and linted with Sudsy (`sudsy.yaml`).

- Check: `sudsy check --format json` (exit 1 on an error or an unformatted file; each diagnostic has `rule`, `range` and `fixes`).
- Fix: `sudsy fix`, then `sudsy check` again. Never edit by hand what `sudsy fix` does.
- Explain a rule: `sudsy rules <id>`. Silence one line: `sudsy-disable-next-line <id>` in a comment; change a level in `sudsy.yaml`.
";

/// Adds the block to the instruction file in `dir`. Returns the file and whether it changed.
pub fn add(dir: &Path) -> Result<(&'static str, bool)> {
    let name = if !dir.join("AGENTS.md").exists() && dir.join("CLAUDE.md").exists() {
        "CLAUDE.md"
    } else {
        "AGENTS.md"
    };
    let path = dir.join(name);
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    if text.contains(START) {
        return Ok((name, false));
    }
    let sep = match text.as_str() {
        "" => "",
        t if t.ends_with("\n\n") => "",
        t if t.ends_with('\n') => "\n",
        _ => "\n\n",
    };
    std::fs::write(&path, format!("{text}{sep}{START}\n{BLOCK}{END}\n"))?;
    Ok((name, true))
}
