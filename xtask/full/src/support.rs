//! The README "Language support" table, generated from the target numbers of the parity runs
//! (docs/ai/design/parity-target.md, Phase 6). The rows are `support.yaml` (language to reference tools);
//! the numbers are the ratchets of `tests/parity/<tool>/differences.yaml`: format = `ratchet` (identical
//! files) over `target-files` (files of the target) summed over the tool's groups, lint = `target`.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// The CLAUDE.md usability bar, in percent.
const BAR: f64 = 85.0;

#[derive(Deserialize)]
struct Row {
    language: String,
    #[serde(default)]
    format: Vec<Tool>,
    #[serde(default)]
    lint: Vec<Tool>,
}

/// A reference tool: its `tests/parity` folder, its name in the table and, for a folder that serves
/// several languages, the ratchet groups of this language.
#[derive(Deserialize)]
struct Tool {
    tool: String,
    name: String,
    #[serde(default)]
    groups: Vec<String>,
}

/// The keys of a `differences.yaml` the table reads.
#[derive(Deserialize, Default)]
struct Ratchets {
    #[serde(default)]
    ratchet: BTreeMap<String, u64>,
    #[serde(default, rename = "target-files")]
    target_files: BTreeMap<String, u64>,
    target: Option<f64>,
}

/// The generated texts of the README: the table with the line under it, the headline count and the
/// note naming the ready languages.
pub struct Support {
    pub table: String,
    pub headline: String,
    pub ready: String,
}

/// The texts of `support_yaml` with the numbers below `root`.
pub fn render(root: &Path, support_yaml: &str) -> Result<Support> {
    let rows: Vec<Row> = serde_saphyr::from_str(support_yaml).context("parsing support.yaml")?;
    let mut out = String::from("| Language | Format | Lint | Status |\n|---|---|---|---|\n");
    let mut ready: Vec<&str> = Vec::new();
    for row in &rows {
        let format: Vec<Option<f64>> = row
            .format
            .iter()
            .map(|t| format_share(root, t))
            .collect::<Result<_>>()?;
        let lint: Vec<Option<f64>> = row
            .lint
            .iter()
            .map(|t| Ok(ratchets(root, &t.tool)?.target))
            .collect::<Result<_>>()?;
        let status = status(row.format.is_empty(), &format, &lint);
        if status.ends_with("Ready") {
            ready.push(&row.language);
        }
        out += &format!(
            "| {} | {} | {} | {status} |\n",
            row.language,
            cell(&row.format, &format),
            cell(&row.lint, &lint),
        );
    }
    out += "\nFormat: the share of real-world files where `sudsy format` gives the same bytes as the tool; Lint: the share of the tool's findings that Sudsy also reports on the same line, with the preset that mirrors the tool. Both leave out only reviewed intentional differences, and both are measured with the tool's default settings. Reproduce a number: `scripts/parity.sh <tool>` (format), `scripts/parity.sh coverage <tool>` (lint). What the numbers leave out: [docs/languages.md](docs/languages.md) and [docs/ai/parity/README.md](docs/ai/parity/README.md). Presets: [configuration](docs/configuration.md#presets).\n";
    let (n, rest) = (ready.len(), rows.len() - ready.len());
    let headline = match n {
        0 => format!("Early support for {rest} languages."),
        _ => format!("Ready for {n} languages, early support for {rest} more."),
    };
    let ready = match ready.split_last() {
        None => "No language is ready yet.".to_string(),
        Some((last, [])) => format!("Ready: {last}. The rest are in progress."),
        Some((last, rest)) => {
            format!(
                "Ready: {} and {last}. The rest are in progress.",
                rest.join(", ")
            )
        }
    };
    Ok(Support {
        table: out,
        headline,
        ready,
    })
}

fn ratchets(root: &Path, tool: &str) -> Result<Ratchets> {
    let path = root
        .join("tests/parity")
        .join(tool)
        .join("differences.yaml");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Ok(Ratchets::default());
    };
    serde_saphyr::from_str(&text).map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))
}

/// Identical over target files across the groups of the tool; `None` until `target-files` exists.
fn format_share(root: &Path, tool: &Tool) -> Result<Option<f64>> {
    let r = ratchets(root, &tool.tool)?;
    let wanted = |g: &&String| tool.groups.is_empty() || tool.groups.contains(g);
    let files: u64 = r
        .target_files
        .keys()
        .filter(wanted)
        .map(|g| r.target_files[g])
        .sum();
    let same: u64 = r
        .target_files
        .keys()
        .filter(wanted)
        .map(|g| r.ratchet.get(g).copied().unwrap_or(0))
        .sum();
    Ok((files > 0).then(|| same as f64 * 100.0 / files as f64))
}

/// One line per tool: bar, percent (rounded down, so the text never reads as passing the bar when it
/// does not) and name; `no reference tool` when the language has none.
fn cell(tools: &[Tool], shares: &[Option<f64>]) -> String {
    if tools.is_empty() {
        return "no reference tool".into();
    }
    tools
        .iter()
        .zip(shares)
        .map(|(t, s)| match s {
            Some(p) => {
                let full = ((p / 10.0).floor() as usize).min(10);
                format!(
                    "{}{} {:.0}% {}",
                    "█".repeat(full),
                    "░".repeat(10 - full),
                    p.floor(),
                    t.name
                )
            }
            None => format!("not measured {}", t.name),
        })
        .collect::<Vec<_>>()
        .join("<br>")
}

fn status(no_formatter: bool, format: &[Option<f64>], lint: &[Option<f64>]) -> &'static str {
    let all: Vec<Option<f64>> = format.iter().chain(lint).copied().collect();
    let ready = all.iter().all(|s| s.is_some_and(|p| p >= BAR));
    match (all.iter().all(Option::is_none), no_formatter, ready) {
        (true, ..) => "not measured",
        (_, true, true) => "Lint only: Ready",
        (_, true, false) => "Lint only: In progress",
        (_, false, true) => "Ready",
        (_, false, false) => "In progress",
    }
}
