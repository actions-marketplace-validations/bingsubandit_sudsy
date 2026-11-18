//! `cargo xtask rename-rule <old> <new> [--dry-run]`: the mechanical part of renaming or merging a
//! rule id. Replaces the id as a whole token (not part of a longer id) in the tracked text files
//! that name Sudsy ids, in the `rule <id>` statuses of `docs/parity/*.yaml` (the keys there are
//! upstream ids and stay), and renames the files and folders named after it. Generated tables
//! and the schema are rewritten by `cargo xtask codegen` afterwards. It leaves alone what it
//! cannot tell from an upstream id (`LEAVE`) and lists the hits there; edit those by hand.

use anyhow::{Context, Result, bail, ensure};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Not rewritten: history, upstream inputs, the plan, and the init mappings and their fixtures,
/// which spell upstream ids.
const LEAVE: &[&str] = &[
    "CHANGELOG.md",
    "docs/design/",
    "docs/parity/",
    "third_party/",
    "bench/",
    "tests/conformance/",
    "tests/fixtures/init/",
    "crates/cli/src/init/",
];

fn is_id_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

/// `text` with each whole-token `old` replaced; the count of replacements.
fn replace_token(text: &str, old: &str, new: &str) -> (String, usize) {
    let (mut out, mut rest, mut n) = (String::new(), text, 0);
    while let Some(at) = rest.find(old) {
        let end = at + old.len();
        let before = rest[..at]
            .chars()
            .next_back()
            .is_none_or(|c| !is_id_char(c));
        let after = rest[end..].chars().next().is_none_or(|c| !is_id_char(c));
        out.push_str(&rest[..at]);
        out.push_str(if before && after { new } else { old });
        n += usize::from(before && after);
        rest = &rest[end..];
    }
    out.push_str(rest);
    (out, n)
}

/// The `rule <id>[, <id>...]` statuses of a catalog; keys (upstream ids) are not touched.
fn replace_status(text: &str, old: &str, new: &str) -> (String, usize) {
    let mut n = 0;
    let lines: Vec<String> = text
        .split('\n')
        .map(|line| match line.split_once(": rule ") {
            Some((key, status)) => {
                let (status, k) = replace_token(status, old, new);
                n += k;
                format!("{key}: rule {status}")
            }
            None => line.to_string(),
        })
        .collect();
    (lines.join("\n"), n)
}

fn tracked(root: &Path) -> Result<Vec<String>> {
    let out = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(root)
        .output()?;
    ensure!(out.status.success(), "git ls-files failed");
    Ok(String::from_utf8(out.stdout)?
        .split('\0')
        .filter(|p| !p.is_empty())
        .map(String::from)
        .collect())
}

/// Renames `old` to `new`; returns what was done, one line each.
pub fn rename_rule(root: &Path, old: &str, new: &str, dry_run: bool) -> Result<Vec<String>> {
    let valid = |id: &str| {
        !id.is_empty()
            && id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    };
    ensure!(
        valid(old) && valid(new) && old != new,
        "ids are `kebab-case` and differ"
    );
    let (mut done, mut left) = (Vec::new(), BTreeMap::new());
    let files = tracked(root)?;
    for rel in &files {
        let path = root.join(rel);
        let Ok(text) = fs::read_to_string(&path) else {
            continue; // binary
        };
        let catalog = rel.starts_with("docs/parity/") && rel.ends_with(".yaml");
        let (changed, n) = if catalog {
            replace_status(&text, old, new)
        } else {
            replace_token(&text, old, new)
        };
        if n == 0 {
            continue;
        }
        let is_leave = LEAVE
            .iter()
            .any(|p| rel.starts_with(p) && !(catalog && *p == "docs/parity/"));
        let corpus = rel.starts_with("tests/parity/") && !rel.ends_with("/sudsy.yaml");
        if is_leave || corpus {
            left.insert(rel.clone(), n);
        } else {
            if !dry_run {
                fs::write(&path, changed).with_context(|| rel.clone())?;
            }
            done.push(format!("{rel}: {n}"));
        }
    }
    for rel in &files {
        let parts: Vec<&str> = rel.split('/').collect();
        let hit = |p: &&str| *p == old || p.strip_prefix(old).is_some_and(|r| r.starts_with('.'));
        if !parts.iter().any(hit) || LEAVE.iter().any(|p| rel.starts_with(p)) {
            continue;
        }
        let moved: Vec<String> = parts
            .iter()
            .map(|p| {
                if hit(p) {
                    p.replacen(old, new, 1)
                } else {
                    p.to_string()
                }
            })
            .collect();
        let target: PathBuf = root.join(moved.join("/"));
        if target.exists() {
            done.push(format!("{rel}: target exists, merge by hand"));
        } else {
            if !dry_run {
                fs::create_dir_all(target.parent().context("parent")?)?;
                fs::rename(root.join(rel), &target)?;
            }
            done.push(format!("{rel} -> {}", moved.join("/")));
        }
    }
    for (rel, n) in left {
        done.push(format!("{rel}: {n} hit(s) NOT rewritten, edit by hand"));
    }
    if done.is_empty() {
        bail!("`{old}` appears nowhere");
    }
    Ok(done)
}
