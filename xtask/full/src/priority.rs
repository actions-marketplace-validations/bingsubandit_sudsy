//! The priority table of docs/ai/parity/queue.md: one row per leg (format or lint of a language's reference
//! tool) under the 85% bar, ranked by weight x points to the bar / difficulty. Everything is read from data that
//! exists: `measured`, `missed`, `ratchet`, `target-files` and the `files` causes of the `differences.yaml`, the
//! catalogs and `support.yaml` (`primary`, `popular`, `blocked`, `corpus`). Format legs count only real-world corpus
//! groups (a name with `real` in it, or listed in the leg's `corpus`; stability groups are all corpora): a leg with
//! only fixtures is not measured, never at the bar.

use crate::queue::{Tool as Parity, gap_counts};
use crate::support::{Row, Tool as Ref};
use languages::catalog::Catalog;
use std::collections::BTreeMap;

/// The usability bar, in percent.
const BAR: u64 = 85;
/// Difficulty of a cause with no existing mapped rule (a new rule, a format gap, files no cause names).
const NEW: u64 = 3;
/// Causes named in a cell before "+N more".
const SHOWN: usize = 4;

enum Outcome {
    AtBar,
    NotMeasured(&'static str),
    Open(Open),
}

struct Open {
    pct: f64,
    need: u64,
    unit: &'static str,
    causes: Vec<(String, u64)>,
    difficulty: u64,
    main: String,
}

struct Leg<'a> {
    language: &'a str,
    kind: &'static str,
    name: String,
    primary: bool,
    weight: u64,
    blocked: Option<&'a str>,
    outcome: Outcome,
}

/// Findings or files still to win, so that `have` of `total` reaches the bar.
fn need(have: u64, total: u64) -> u64 {
    (BAR * total).div_ceil(100).saturating_sub(have)
}

/// The causes (largest first) that add up to `need`; a rest no cause explains is one more cause.
fn pick(
    mut causes: Vec<(String, u64)>,
    need: u64,
    cost: &dyn Fn(&str) -> u64,
) -> (Vec<(String, u64)>, u64) {
    causes.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let (mut sum, mut out, mut difficulty) = (0, Vec::new(), 0);
    for (name, n) in causes {
        if sum >= need {
            break;
        }
        sum += n;
        difficulty += cost(&name);
        out.push((name, n));
    }
    if sum < need {
        out.push(("unclassified".into(), need - sum));
        difficulty += NEW;
    }
    (out, difficulty)
}

fn format_leg(tool: &Ref, parity: Option<&Parity>, stability: bool) -> Outcome {
    let Some(t) = parity else {
        return Outcome::NotMeasured("no differences.yaml");
    };
    let real = |g: &str| {
        (tool.groups.is_empty() || tool.groups.iter().any(|x| x == g))
            && (stability || g.contains("real") || tool.corpus.iter().any(|x| x == g))
    };
    let groups: Vec<&String> = t.target_files.keys().filter(|g| real(g)).collect();
    let files: u64 = groups.iter().map(|g| t.target_files[*g]).sum();
    if files == 0 {
        return Outcome::NotMeasured("no real-world group");
    }
    let same: u64 = groups
        .iter()
        .map(|g| t.ratchet.get(*g).copied().unwrap_or(0))
        .sum();
    if need(same, files) == 0 {
        return Outcome::AtBar;
    }
    // Files named one by one belong to the leg when it spans every group of the tool.
    let all = t.target_files.keys().all(|g| real(g));
    let causes = gap_counts(t, &real, all)
        .into_iter()
        .filter_map(|(n, c)| c.filter(|c| *c > 0).map(|c| (n.to_string(), c)))
        .collect();
    let need = need(same, files);
    let (causes, difficulty) = pick(causes, need, &|_| NEW);
    Outcome::Open(Open {
        pct: same as f64 * 100.0 / files as f64,
        need,
        unit: "files",
        causes,
        difficulty,
        main: "-".into(),
    })
}

fn lint_leg(tool: &Ref, parity: Option<&Parity>, catalogs: &[Catalog]) -> Outcome {
    let Some(m) = parity.and_then(|t| t.measured.as_ref().map(|m| (t, m))) else {
        return Outcome::NotMeasured("no `measured` coverage run");
    };
    let (t, m) = m;
    if m.total == 0 {
        return Outcome::NotMeasured("no findings in the corpus");
    }
    if need(m.covered, m.total) == 0 {
        return Outcome::AtBar;
    }
    let catalog = catalogs
        .iter()
        .find(|c| c.tool.replace("plugin-", "") == tool.tool);
    // A missed rule with a mapped Sudsy rule is a fix (1); an unmapped one is a new rule.
    let cost = |rule: &str| {
        let mapped = catalog.is_some_and(|c| {
            c.entries.iter().any(|e| {
                (e.id == rule || e.id.split([',', '/']).any(|p| p.trim() == rule))
                    && e.rules().next().is_some()
            })
        });
        if mapped { 1 } else { NEW }
    };
    let need = need(m.covered, m.total);
    let missed = t.missed.iter().map(|(r, n)| (r.clone(), *n)).collect();
    let (causes, difficulty) = pick(missed, need, &cost);
    Outcome::Open(Open {
        pct: m.covered as f64 * 100.0 / m.total as f64,
        need,
        unit: "findings",
        causes,
        difficulty,
        main: m.main.clone(),
    })
}

fn legs<'a>(
    rows: &'a [Row],
    tools: &BTreeMap<String, Parity>,
    catalogs: &[Catalog],
) -> Vec<Leg<'a>> {
    let mut out = Vec::new();
    let weight = |i: usize, t: &Ref| match (i, t.popular) {
        (0, _) => 3,
        (_, true) => 2,
        _ => 1,
    };
    for row in rows {
        for (i, t) in row.format.iter().enumerate() {
            out.push(Leg {
                language: &row.language,
                kind: "format",
                name: t.name.clone(),
                primary: i == 0,
                weight: weight(i, t),
                blocked: t.blocked.as_deref(),
                outcome: format_leg(t, tools.get(&t.tool), false),
            });
        }
        if !row.stability.is_empty() {
            let t = Ref {
                tool: "stability".into(),
                name: "stable".into(),
                groups: row.stability.clone(),
                ..Ref::default()
            };
            out.push(Leg {
                language: &row.language,
                kind: "format",
                name: t.name.clone(),
                primary: true,
                weight: 3,
                blocked: None,
                outcome: format_leg(&t, tools.get("stability"), true),
            });
        }
        for (i, t) in row.lint.iter().enumerate() {
            out.push(Leg {
                language: &row.language,
                kind: "lint",
                name: t.name.clone(),
                primary: i == 0,
                weight: weight(i, t),
                blocked: t.blocked.as_deref(),
                outcome: lint_leg(t, tools.get(&t.tool), catalogs),
            });
        }
    }
    out
}

fn causes(o: &Open) -> String {
    let shown: Vec<String> = o
        .causes
        .iter()
        .take(SHOWN)
        .map(|(n, c)| format!("`{n}` {c}"))
        .collect();
    let more = o.causes.len().saturating_sub(SHOWN);
    let more = if more > 0 {
        format!(" +{more} more")
    } else {
        String::new()
    };
    format!("{}{more}", shown.join(", "))
}

/// The section at the top of the queue.
pub(crate) fn table(
    rows: &[Row],
    tools: &BTreeMap<String, Parity>,
    catalogs: &[Catalog],
) -> String {
    let legs = legs(rows, tools, catalogs);
    let mut open: Vec<(f64, String)> = Vec::new();
    let mut blocked = String::new();
    let mut unmeasured = Vec::new();
    for l in &legs {
        let leg = format!("{} {}", l.kind, l.name);
        match &l.outcome {
            Outcome::AtBar => {}
            Outcome::NotMeasured(why) => unmeasured.push(format!(
                "{} {leg} ({why}{})",
                l.language,
                l.blocked.map_or("", |_| ", blocked")
            )),
            Outcome::Open(o) => {
                let points = BAR as f64 - o.pct;
                if let Some(why) = l.blocked {
                    blocked += &format!(
                        "| {} | {leg} | {:.1}% | {points:.1} | {why} |\n",
                        l.language, o.pct
                    );
                    continue;
                }
                let score = l.weight as f64 * points / o.difficulty as f64;
                open.push((
                    score,
                    format!(
                        "| {} | {leg} | {} | {:.1}% | {points:.1} | {} {} | {} | {} | {score:.2} | {} |\n",
                        l.language,
                        l.weight,
                        o.pct,
                        o.need,
                        o.unit,
                        causes(o),
                        o.difficulty,
                        o.main
                    ),
                ));
            }
        }
    }
    open.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut out = String::from(
        "## Priority\n\nWeight: the primary tool of a leg 3, a popular secondary 2, any other 1. Points: to the 85% bar. Needs: the findings (lint) or files (format, real-world corpus groups only) still to win; causes: the largest until the need is met (a rest no cause names is `unclassified`). Difficulty: per cause 1 when its rule exists and is mapped in the catalog, 3 for a new rule, a format gap or an unclassified rest. Score: weight x points / difficulty. A leg at the bar is not listed.\n\n| Language | Leg | Weight | Now | Points | Needs | Causes | Difficulty | Score | Measured on |\n|---|---|---|---|---|---|---|---|---|---|\n",
    );
    for (_, row) in &open {
        out += row;
    }
    if !blocked.is_empty() {
        out += &format!(
            "\nBlocked on a decision:\n\n| Language | Leg | Now | Points | Reason |\n|---|---|---|---|---|\n{blocked}"
        );
    }
    if !unmeasured.is_empty() {
        out += &format!("\nNot measured: {}.\n", unmeasured.join("; "));
    }
    let holding: Vec<String> = legs
        .iter()
        .filter(|l| l.primary && l.blocked.is_none())
        .filter(|l| !matches!(l.outcome, Outcome::AtBar))
        .map(|l| format!("{} {} {}", l.language, l.kind, l.name))
        .collect();
    out += &if holding.is_empty() {
        "\nNew language: open, every primary leg is at the bar or blocked.\n\n".to_string()
    } else {
        format!(
            "\nNew language: locked until every primary leg is at the bar or blocked. Held by {}: {}.\n\n",
            holding.len(),
            holding.join("; ")
        )
    };
    out
}
