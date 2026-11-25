//! The check that no measured share is typed in prose: a percentage on a line that names a reference tool must sit in
//! a generated block (`<!-- generated:NAME -->` ... `<!-- /generated -->`), where the number comes from the parity
//! ratchets, or the line carries the date of its one-off measurement (`2026-11-25`). The files read: the README, the
//! pages of `docs/`, `docs/ai/formatting/` and `docs/ai/parity/`. The history files (`bench/`, `CHANGELOG.md`, the dated
//! design plans in `docs/ai/design/`), the working notes (`docs/ai/development/`, `docs/ai/roadmap.md`) and the
//! generated `docs/ai/parity/queue.md` are not read.

use anyhow::{Result, bail};
use std::path::Path;

const END: &str = "<!-- /generated -->";

/// The files of the check, relative to the repository root.
fn files(root: &Path) -> Result<Vec<String>> {
    let mut out = vec!["README.md".to_string()];
    for dir in ["docs", "docs/ai/formatting", "docs/ai/parity"] {
        let mut names: Vec<String> = std::fs::read_dir(root.join(dir))?
            .filter_map(|e| e.ok()?.file_name().into_string().ok())
            .filter(|n| n.ends_with(".md") && n != "queue.md")
            .collect();
        names.sort();
        out.extend(names.into_iter().map(|n| format!("{dir}/{n}")));
    }
    Ok(out)
}

/// The names of the reference tools: the first word and the whole of every tool name of `support.yaml`, and the
/// name of every catalog of `docs/ai/parity`.
pub(crate) fn tool_names(root: &Path, rows: &[crate::support::Row]) -> Result<Vec<String>> {
    let mut names: Vec<String> = rows
        .iter()
        .flat_map(|r| r.format.iter().chain(&r.lint))
        .flat_map(|t| {
            [
                t.name.split_whitespace().next().unwrap_or("").to_string(),
                t.name.clone(),
            ]
        })
        .collect();
    for file in std::fs::read_dir(root.join("docs/ai/parity"))? {
        let file = file?.path();
        if file.extension().is_some_and(|e| e == "yaml") {
            names.extend(file.file_stem().map(|s| s.to_string_lossy().into_owned()));
        }
    }
    names.retain(|n| !n.is_empty());
    names.sort();
    names.dedup();
    Ok(names)
}

/// Fails for every typed share of the files, naming file, line and tool.
pub(crate) fn check(root: &Path, names: &[String]) -> Result<()> {
    let mut found = Vec::new();
    for rel in files(root)? {
        let text = std::fs::read_to_string(root.join(&rel))?;
        found.extend(
            typed_shares(&text, names)
                .into_iter()
                .map(|(line, tool)| format!("{rel}:{line}: a percentage next to `{tool}`")),
        );
    }
    if !found.is_empty() {
        bail!(
            "typed shares outside generated blocks (put the number in a generated block, link to the table, or date the one-off measurement on the line):\n{}",
            found.join("\n")
        );
    }
    Ok(())
}

/// The lines of `text` outside generated blocks that hold a percentage and name a tool, as (line, tool); a line
/// with a date is a dated one-off measurement and passes.
pub(crate) fn typed_shares(text: &str, names: &[String]) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut inside = false;
    for (i, line) in text.lines().enumerate() {
        if line.contains("<!-- generated:") {
            inside = true;
        }
        let skip = inside;
        if line.contains(END) {
            inside = false;
        }
        if skip || !has_percent(line) || has_date(line) {
            continue;
        }
        if let Some(tool) = names.iter().find(|n| names_word(line, n)) {
            out.push((i + 1, tool.clone()));
        }
    }
    out
}

/// A number followed by `%`.
fn has_percent(line: &str) -> bool {
    let b = line.as_bytes();
    (1..b.len()).any(|i| b[i] == b'%' && b[i - 1].is_ascii_digit())
}

/// `YYYY-MM-DD`.
fn has_date(line: &str) -> bool {
    let b = line.as_bytes();
    b.windows(10).any(|w| {
        w.iter().enumerate().all(|(i, c)| match i {
            4 | 7 => *c == b'-',
            _ => c.is_ascii_digit(),
        })
    })
}

/// The line holds `name` as a word: not inside a longer name (`ruff` in `ruffle`, `eslint` in `eslint-plugin-vue`).
fn names_word(line: &str, name: &str) -> bool {
    let word = |c: char| c.is_alphanumeric() || c == '_' || c == '-';
    line.match_indices(name).any(|(at, _)| {
        !line[..at].chars().next_back().is_some_and(word)
            && !line[at + name.len()..].chars().next().is_some_and(word)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names() -> Vec<String> {
        vec!["ruff".into(), "eslint-plugin-vue".into()]
    }

    #[test]
    fn a_percentage_next_to_a_tool_is_found() {
        let text = "Intro\nruff covers 95.6% of findings.\nOther 5% prose.\n";
        assert_eq!(typed_shares(text, &names()), [(2, "ruff".to_string())]);
    }

    #[test]
    fn a_generated_block_a_date_and_a_longer_name_pass() {
        let text = "<!-- generated:t -->\nruff 95%\n<!-- /generated -->\nruff 90% (measured 2026-11-25)\nruffle 90%\nruff-ish 5%\n";
        assert!(typed_shares(text, &names()).is_empty());
        // Text after the block counts again; an inline block hides only its own line.
        let after = "<!-- generated:t -->x ruff 9%<!-- /generated --> and ruff 8%\nruff 7%\n";
        assert_eq!(typed_shares(after, &names()), [(2, "ruff".to_string())]);
    }

    #[test]
    fn the_repository_has_no_typed_share() {
        // The same check as `cargo xtask codegen --check`, on the files of the checkout.
        let root = xtask::repo_root();
        let support = std::fs::read_to_string(root.join("xtask/full/support.yaml")).unwrap();
        let rows = crate::support::rows(&support).unwrap();
        check(&root, &tool_names(&root, &rows).unwrap()).unwrap();
    }
}
