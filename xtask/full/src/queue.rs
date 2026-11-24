//! The parity queue `docs/ai/parity/queue.md`: per tool the open work, from `tests/parity/<tool>/differences.yaml`
//! (docs/ai/development/parity.md): the unclassified count with its share, the `gap` names ranked by files, the lint rules
//! ranked by missed findings, each with its `fixes:` note.

use anyhow::Result;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Deserialize, Default)]
pub(crate) struct Tool {
    #[serde(default)]
    pub differences: BTreeMap<String, String>,
    #[serde(default)]
    pub files: BTreeMap<String, String>,
    #[serde(default)]
    fixes: BTreeMap<String, String>,
    #[serde(default)]
    sets: Vec<String>,
    #[serde(default)]
    pub missed: BTreeMap<String, u64>,
    pub measured: Option<Measured>,
    #[serde(default)]
    pub ratchet: BTreeMap<String, u64>,
    #[serde(default, rename = "target-files")]
    pub target_files: BTreeMap<String, u64>,
    unclassified: Option<u64>,
    pub target: Option<f64>,
}

/// What the last `COVERAGE_WRITE=1` run of the coverage counted; `main` is the commit its tree is based on.
#[derive(Deserialize)]
pub(crate) struct Measured {
    pub covered: u64,
    pub total: u64,
    pub main: String,
}

/// The rules the queue lists per tool; the rest is one line with its sum.
const TOP: usize = 15;

/// The gaps with the files that name them as first cause (`None`: listed in a group without a count), most files first.
fn gaps(t: &Tool) -> Vec<(&str, Option<u64>)> {
    gap_counts(t, &|_| true, true)
}

/// `gaps` over the ratchet groups `group_ok` accepts; the files named one by one (paths) count when `paths` is set.
pub(crate) fn gap_counts<'a>(
    t: &'a Tool,
    group_ok: &dyn Fn(&str) -> bool,
    paths: bool,
) -> Vec<(&'a str, Option<u64>)> {
    let mut counts: BTreeMap<&str, Option<u64>> = BTreeMap::new();
    for (key, names) in &t.files {
        if key.contains(' ') {
            continue; // a lint finding
        }
        if !(if t.ratchet.contains_key(key) {
            group_ok(key)
        } else {
            paths
        }) {
            continue;
        }
        for (i, name) in names.split(',').map(str::trim).enumerate() {
            let (name, n) = name
                .split_once(':')
                .map_or((name, None), |(a, n)| (a, n.parse().ok()));
            let entry = counts.entry(name).or_insert(None);
            // A file of its own has its first name as the cause; a group says so with `name:N`.
            let add = if t.ratchet.contains_key(key) {
                n
            } else {
                (i == 0).then_some(1)
            };
            if let Some(add) = add {
                *entry = Some(entry.unwrap_or(0) + add);
            }
        }
    }
    let mut out: Vec<_> = counts
        .into_iter()
        .filter(|(n, _)| t.differences.get(*n).is_some_and(|d| d.starts_with("gap")))
        .collect();
    out.sort_by(|a, b| b.1.unwrap_or(0).cmp(&a.1.unwrap_or(0)).then(a.0.cmp(b.0)));
    out
}

fn section(name: &str, t: &Tool) -> String {
    let mut out = String::new();
    let differing: u64 = t
        .ratchet
        .iter()
        .map(|(g, r)| {
            t.target_files
                .get(g)
                .copied()
                .unwrap_or(0)
                .saturating_sub(*r)
        })
        .sum();
    if let (true, Some(u)) = (differing > 0, t.unclassified) {
        out += &format!(
            "Format: unclassified {u} of {differing} differing files ({}%).\n",
            (100 * u + differing / 2) / differing
        );
    }
    let note = |key: &str| {
        t.fixes
            .get(key)
            .map_or(String::new(), |f| format!(" Fix: {f}"))
    };
    let gaps = gaps(t);
    for (rank, (gap, n)) in gaps.iter().enumerate() {
        let text = t.differences[*gap].trim_start_matches("gap").trim();
        let n = n.map_or("-".to_string(), |n| n.to_string());
        out += &format!("{}. `{gap}` ({n} files): {text}{}\n", rank + 1, note(gap));
    }
    if !t.sets.is_empty() {
        out += "Ranked sets (greedy, `scripts/parity_gaps.py`: fix the causes from the top together; files equal after each):\n";
        for (i, s) in t.sets.iter().take(TOP).enumerate() {
            out += &format!("{}. {s}\n", i + 1);
        }
    }
    if !t.missed.is_empty() {
        let mut rules: Vec<_> = t.missed.iter().collect();
        rules.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        let share = t.target.map_or("-".into(), |c| c.to_string());
        out += &match &t.measured {
            Some(m) => format!(
                "Lint: target {share}% ({} of {}, {} missed, measured on {}); rules by missed findings:\n",
                m.covered,
                m.total,
                m.total - m.covered,
                m.main
            ),
            None => format!(
                "Lint: target {share}%; rules by missed findings (an incomplete list: not re-measured with the layout findings):\n"
            ),
        };
        for (rank, (rule, n)) in rules.iter().take(TOP).enumerate() {
            out += &format!("{}. `{rule}` ({n} missed){}\n", rank + 1, note(rule));
        }
        if let Some(rest) = rules.get(TOP..).filter(|r| !r.is_empty()) {
            let sum: u64 = rest.iter().map(|(_, n)| **n).sum();
            out += &format!("\n{} more rules: {sum} missed.\n", rest.len());
        }
    }
    if out.is_empty() {
        out
    } else {
        format!("## {name}\n\n{out}\n")
    }
}

pub fn generate() -> Result<(PathBuf, String)> {
    let rows = crate::support::rows(include_str!("../support.yaml"))?;
    let catalogs = crate::docs::catalogs()?;
    let root = xtask::repo_root();
    let mut tools = BTreeMap::new();
    for folder in std::fs::read_dir(root.join("tests/parity"))? {
        let path = folder?.path();
        let Ok(text) = std::fs::read_to_string(path.join("differences.yaml")) else {
            continue;
        };
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let tool: Tool = serde_saphyr::from_str(&text)
            .map_err(|e| anyhow::anyhow!("tests/parity/{name}/differences.yaml: {e}"))?;
        if let Some(m) = &tool.measured {
            let missed: u64 = tool.missed.values().sum();
            anyhow::ensure!(
                m.covered + missed == m.total,
                "tests/parity/{name}/differences.yaml: {} covered + {missed} missed is not the {} of `measured`",
                m.covered,
                m.total
            );
        }
        tools.insert(name, tool);
    }
    let mut text = String::from(
        "# Parity queue\n\nGenerated by `cargo xtask codegen` from `tests/parity/<tool>/differences.yaml`; do not edit. Per tool: the unclassified count (differing files no `name:N` of `files` explains), the `gap` names by files (`-`: named in a group, not counted), the lint rules by missed findings, each with its `fixes:` note. How to work it: [../development/parity.md](../development/parity.md#the-queue).\n\n",
    );
    text += &crate::priority::table(&rows, &tools, &catalogs);
    for (name, tool) in &tools {
        text += &section(name, tool);
    }
    Ok(("docs/ai/parity/queue.md".into(), text))
}
