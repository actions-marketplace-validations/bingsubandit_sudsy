//! The new-language rows of the priority table (docs/ai/parity/queue.md), from `languages.yaml`: per language
//! its demand, parser cost and, per leg (format, lint), the alternatives of the reference tool with gain, reach
//! and cost. Each plan (lint, format, both) scores `4 x 85 x Demand x sum(Gain x Reach) / (20 x (Parser + sum
//! Cost))`; every alternative of a leg competes and the best wins. A number written `"~0.6"` is an estimate; a
//! score that uses one is printed with `~` and only the measuring of the language may be picked from it.

use crate::support::Row;
use anyhow::{Result, bail, ensure};
use serde::{Deserialize, Deserializer};

/// Weight of the primary leg in the formula.
const WEIGHT: f64 = 4.0;
/// The usability bar, in percent.
const BAR: f64 = 85.0;
/// Scale of the formula: the cost of a leg of cost 1 and parser 1 at gain x reach 1 scores `WEIGHT x BAR / 20 / 2`.
const NORM: f64 = 20.0;
/// Pushers of the top language of the GitHub Innovation Graph (HTML, 2026 Q2, summed over economies).
const GH_TOP: f64 = 8_329_335.0;
/// Share (%) of respondents of the Stack Overflow Developer Survey 2026 who worked with the top language (JavaScript).
const SO_TOP: f64 = 62.8;
/// Median over the 42 languages both sources list of `sqrt(so / SO_TOP) / sqrt(gh / GH_TOP)`: how much higher the survey
/// level is than the pusher level. Stands in for the source that does not list a language.
const SO_PER_GH: f64 = 2.27;
/// Rows of the table; the other entries follow as one line of names and scores.
const TOP: usize = 25;
/// Lowest demand.
const FLOOR: f64 = 0.1;
/// The values `gain` takes (docs/ai/parity/queue.md: official, third party, slow or own runtime, none).
const GAINS: [f64; 4] = [0.1, 0.4, 0.7, 1.0];

/// A number of the file; `"~0.6"` marks an estimate.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct V {
    pub v: f64,
    pub est: bool,
}

impl<'de> Deserialize<'de> for V {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Num(f64),
            Text(String),
        }
        match Raw::deserialize(d)? {
            Raw::Num(v) => Ok(V { v, est: false }),
            Raw::Text(s) => {
                let est = s.starts_with('~');
                s.trim_start_matches('~')
                    .parse()
                    .map(|v| V { v, est })
                    .map_err(|_| serde::de::Error::custom(format!("`{s}` is not a number")))
            }
        }
    }
}

impl V {
    fn show(self) -> String {
        format!("{}{}", if self.est { "~" } else { "" }, self.v)
    }
}

/// Demand 0.1-1 from the raw values of the two sources: per source `sqrt(x / top)`, a missing source estimated from the
/// listed one by `SO_PER_GH` (the demand is then an estimate), the mean of the two, floored at `FLOOR`. A language in no
/// source is `FLOOR`, an estimate.
pub(crate) fn demand(gh: Option<f64>, so: Option<f64>) -> V {
    let g = gh.map(|x| (x / GH_TOP).sqrt());
    let s = so.map(|x| (x / SO_TOP).sqrt());
    let (g, s, est) = match (g, s) {
        (Some(g), Some(s)) => (g, s, false),
        (Some(g), None) => (g, g * SO_PER_GH, true),
        (None, Some(s)) => (s / SO_PER_GH, s, true),
        (None, None) => {
            return V {
                v: FLOOR,
                est: true,
            };
        }
    };
    V {
        v: ((g + s) / 2.0).clamp(FLOOR, 1.0),
        est,
    }
}

/// One way to cover a leg: a reference tool with its gain, reach and cost.
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub(crate) struct Alt {
    pub tool: String,
    pub gain: V,
    pub reach: V,
    pub cost: V,
    /// One line: why these values.
    pub why: String,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub(crate) struct Entry {
    pub language: String,
    /// Pushers of the GitHub Innovation Graph (2026 Q2, summed over economies), when it lists the language.
    pub gh: Option<f64>,
    /// Share (%) of the Stack Overflow survey respondents who worked with it, when the survey lists it.
    pub so: Option<f64>,
    #[serde(skip)]
    pub demand: V,
    /// Absent for a language that is supported with a leg missing (the parser is paid).
    pub parser: Option<V>,
    #[serde(default)]
    pub format: Vec<Alt>,
    #[serde(default)]
    pub lint: Vec<Alt>,
    /// One line shown in the Notes column.
    #[serde(default)]
    pub note: Option<String>,
    /// Legs (`format`, `lint`) the user put off: no row, they hold nothing.
    #[serde(default)]
    pub deferred: Vec<String>,
    #[serde(skip)]
    pub paid: bool,
}

/// The entries of `languages.yaml`, checked against the scales and against the supported languages.
pub(crate) fn entries(yaml: &str, support: &[Row]) -> Result<Vec<Entry>> {
    let mut list: Vec<Entry> =
        serde_saphyr::from_str(yaml).map_err(|e| anyhow::anyhow!("parsing languages.yaml: {e}"))?;
    let mut seen = Vec::new();
    for e in &mut list {
        let name = e.language.clone();
        ensure!(!seen.contains(&name), "languages.yaml: `{name}` twice");
        seen.push(name.clone());
        for (src, v, top) in [("gh", e.gh, GH_TOP), ("so", e.so, SO_TOP)] {
            ensure!(
                v.is_none_or(|v| v > 0.0 && v <= top),
                "languages.yaml: `{name}` {src} {v:?} is not above 0 and at most {top}"
            );
        }
        // A supported language keeps its raw values in support.yaml (one place for every popularity).
        let row = support.iter().find(|r| r.language == name);
        e.demand = row.map_or_else(|| demand(e.gh, e.so), Row::source_popularity);
        for leg in &e.deferred {
            ensure!(
                leg == "format" || leg == "lint",
                "languages.yaml: `{name}` defers unknown leg `{leg}`"
            );
        }
        e.paid = row.is_some();
        match (row, e.parser) {
            (Some(_), Some(_)) => {
                bail!("languages.yaml: `{name}` is supported, its parser is paid: drop `parser`")
            }
            (None, None) => bail!("languages.yaml: `{name}` has no `parser`"),
            (None, Some(p)) => {
                ensure!(p.v > 0.0, "languages.yaml: `{name}` parser must be above 0")
            }
            _ => {}
        }
        if let Some(r) = row {
            let has_format = !r.format.is_empty() || !r.stability.is_empty();
            let has_lint = !r.lint.is_empty();
            ensure!(
                !(has_format && has_lint),
                "languages.yaml: `{name}` is fully supported"
            );
            ensure!(
                !(has_format && !e.format.is_empty()),
                "languages.yaml: `{name}` already has a format leg"
            );
            ensure!(
                !(has_lint && !e.lint.is_empty()),
                "languages.yaml: `{name}` already has a lint leg"
            );
        }
        ensure!(
            row.is_none() || (e.gh.is_none() && e.so.is_none()),
            "languages.yaml: `{name}` is supported: its `gh` and `so` are the `popularity` of its row in support.yaml"
        );
        for (leg, alts) in [("format", &e.format), ("lint", &e.lint)] {
            for a in alts {
                let at = format!("languages.yaml: `{name}` {leg} `{}`", a.tool);
                ensure!(
                    !a.tool.trim().is_empty() && !a.why.trim().is_empty(),
                    "{at}: tool and why are required"
                );
                ensure!(
                    GAINS.contains(&a.gain.v),
                    "{at}: gain {} is not one of {GAINS:?}",
                    a.gain.v
                );
                ensure!(
                    (0.0..=1.0).contains(&a.reach.v),
                    "{at}: reach {} is not 0-1",
                    a.reach.v
                );
                ensure!(a.cost.v > 0.0, "{at}: cost must be above 0");
            }
        }
    }
    Ok(list)
}

/// A scored plan: its legs with the alternative chosen for each.
pub(crate) struct Plan<'a> {
    pub name: &'static str,
    pub score: f64,
    pub est: bool,
    pub picks: Vec<(&'static str, &'a Alt)>,
}

/// `4 x 85 x Demand x sum(Gain x Reach) / (20 x (Parser + sum Cost))`.
fn score(demand: f64, parser: f64, picks: &[(&'static str, &Alt)]) -> f64 {
    let value: f64 = picks.iter().map(|(_, a)| a.gain.v * a.reach.v).sum();
    let cost: f64 = picks.iter().map(|(_, a)| a.cost.v).sum();
    WEIGHT * BAR * demand * value / (NORM * (parser + cost))
}

fn leg<'a>(e: &'a Entry, name: &'static str) -> &'a [Alt] {
    if e.deferred.iter().any(|d| d == name) {
        &[]
    } else if name == "lint" {
        &e.lint
    } else {
        &e.format
    }
}

/// Lint, format and both, each with the best alternative of each leg; a plan needs every leg it names.
pub(crate) fn plans(e: &Entry) -> Vec<Plan<'_>> {
    let parser = e.parser.map_or(0.0, |p| p.v);
    let (lint, format) = (leg(e, "lint"), leg(e, "format"));
    let mut out = Vec::new();
    for (name, legs) in [
        ("lint", vec![("lint", lint)]),
        ("format", vec![("format", format)]),
        ("both", vec![("lint", lint), ("format", format)]),
    ] {
        if legs.iter().any(|(_, a)| a.is_empty()) {
            continue;
        }
        // Every combination of one alternative per leg.
        let mut combos: Vec<Vec<(&'static str, &Alt)>> = vec![vec![]];
        for (leg, alts) in &legs {
            combos = combos
                .into_iter()
                .flat_map(|c| {
                    alts.iter().map(move |a| {
                        let mut c = c.clone();
                        c.push((*leg, a));
                        c
                    })
                })
                .collect();
        }
        let best = combos
            .into_iter()
            .map(|picks| (score(e.demand.v, parser, &picks), picks))
            .max_by(|a, b| a.0.total_cmp(&b.0));
        if let Some((score, picks)) = best {
            let est = e.demand.est
                || e.parser.is_some_and(|p| p.est)
                || picks
                    .iter()
                    .any(|(_, a)| a.gain.est || a.reach.est || a.cost.est);
            out.push(Plan {
                name,
                score,
                est,
                picks,
            });
        }
    }
    out
}

fn cell(p: Option<&Plan>) -> String {
    p.map_or("-".into(), |p| {
        format!("{:.2}{}", p.score, if p.est { "~" } else { "" })
    })
}

/// The best plan, the first of equals.
fn best<'a, 'b>(plans: &'b [Plan<'a>]) -> Option<&'b Plan<'a>> {
    plans.iter().fold(None, |b, p| match b {
        Some(b) if b.score >= p.score => Some(b),
        _ => Some(p),
    })
}

/// The section of the priority table: the rule for a free slot, the primary legs still under the bar and one row
/// per language, the best suggestion first.
pub(crate) fn section(list: &[Entry], holding: &[String]) -> String {
    let mut rows: Vec<(f64, String, String)> = Vec::new();
    for e in list {
        let plans = plans(e);
        let by = |n: &str| plans.iter().find(|p| p.name == n);
        let (suggestion, notes) = match best(&plans) {
            None => ("-".into(), "no leg left to build".to_string()),
            Some(b) => {
                let tools: Vec<String> = b
                    .picks
                    .iter()
                    .map(|(l, a)| format!("{l} {}", a.tool))
                    .collect();
                let mut notes: Vec<String> = b
                    .picks
                    .iter()
                    .filter(|(_, a)| a.reach.v < BAR / 100.0)
                    .map(|(l, a)| {
                        format!(
                            "cannot reach the bar: {l} {} reach {}",
                            a.tool,
                            a.reach.show()
                        )
                    })
                    .collect();
                notes.extend(e.note.clone());
                if b.est {
                    notes.push("estimate: pick only to measure it".into());
                }
                (
                    format!("{}: {}", b.name, tools.join(" + ")),
                    notes.join("; "),
                )
            }
        };
        let parser = if e.paid {
            "0 (paid)".to_string()
        } else {
            e.parser.map_or("-".into(), V::show)
        };
        rows.push((
            best(&plans).map_or(-1.0, |b| b.score),
            format!(
                "| {} | {}{} | {parser} | {} | {} | {} | {suggestion} | {notes} |\n",
                e.language,
                format_args!("{:.2}", e.demand.v),
                if e.demand.est { "~" } else { "" },
                cell(by("lint")),
                cell(by("format")),
                cell(by("both")),
            ),
            format!("{} {}", e.language, cell(best(&plans))),
        ));
    }
    rows.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut out = format!(
        "\nNew languages (`xtask/full/languages.yaml`). Score of a plan = {WEIGHT} x {BAR} x Demand x sum(Gain x Reach) / ({NORM} x (Parser + sum Cost)), summed over the legs of the plan (lint, format, both); every alternative of a leg competes and the best wins; a leg of reach under 0.85 is scored too. `~` marks an estimate, and a score with one is `~`: a row with an estimate may only be picked to measure it (its values), not to build it. Rule for a free slot: a primary leg under the bar that an agent works on counts as handled; the slot goes to a primary leg under the bar without an agent first, else to the best new-language row, before any secondary-tool work. This table cannot see who works on what. Primary legs under the bar now: {}.\n\n| Language | Demand | Parser | Lint | Format | Both | Suggestion | Notes |\n|---|---|---|---|---|---|---|---|\n",
        if holding.is_empty() {
            "none".to_string()
        } else {
            holding.join("; ")
        }
    );
    for (_, row, _) in rows.iter().take(TOP) {
        out += row;
    }
    if rows.len() > TOP {
        let rest: Vec<&str> = rows[TOP..].iter().map(|r| r.2.as_str()).collect();
        out += &format!(
            "\nThe other {} entries of the file, each with the score of its best plan: {}.\n",
            rest.len(),
            rest.join("; ")
        );
    }
    out + "\n"
}
