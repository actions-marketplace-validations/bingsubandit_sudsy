//! One entry per rule id, gathered once: `docs/ai/rules/rules.json` (what the documentation site and
//! the VS Code extension read) and the rule table of `docs/ai/rules/builtin.md` are both made from it.

use core_rules::{Check, Extensions, Opt, RuleMeta};
use languages::catalog::{self, Catalog, Upstream};
use provider_api::LanguageProvider;
use provider_api::fixtures::{self, Example};
use serde_json::{Value, json};

pub struct RuleDoc {
    pub meta: &'static RuleMeta,
    /// `all` for the shared rules.
    pub languages: Vec<String>,
    /// Presets that turn the rule on.
    pub presets: Vec<String>,
    pub upstream: Vec<Upstream>,
    /// The first case of the rule's fixtures that reports it (the one in the rule's own folder
    /// when there is one), with its language.
    pub example: Option<(String, Example)>,
    /// Languages whose rule of this id has the same own `why` as the first one: the text belongs
    /// in `crates/core-rules/texts/why.txt` once.
    pub same_why: Vec<String>,
    whys: Vec<(String, String)>,
}

/// Every rule, shared rules first, then by language in provider order, then by id.
pub fn collect(providers: &[Box<dyn LanguageProvider>], catalogs: &[Catalog]) -> Vec<RuleDoc> {
    let rules = languages::rules(providers, &Extensions::default());
    let mut presets = languages::rule_presets(providers);
    let mut examples: Vec<(String, Example)> = Vec::new();
    for p in providers {
        let language = p.language().to_string();
        examples.extend(
            fixtures::examples(p.as_ref())
                .into_iter()
                .map(|e| (language.clone(), e)),
        );
    }
    let mut docs: Vec<RuleDoc> = Vec::new();
    for (language, rule) in rules {
        let meta = rule.meta();
        if let Some(doc) = docs.last_mut().filter(|d| d.meta.id == meta.id) {
            // The shared pattern rules are one rule per language with one text; two rules of one id
            // with the same `why` and another `description` are two copies of the prose.
            let (why, description) = (
                meta.own_why().into_owned(),
                meta.description_text().into_owned(),
            );
            if !why.trim().is_empty()
                && doc.whys.iter().any(|(w, d)| *w == why && *d != description)
            {
                doc.same_why.push(language.clone());
            }
            doc.whys.push((why, description));
            doc.languages.push(language);
            continue;
        }
        let hits = || {
            examples
                .iter()
                .filter(|(_, e)| e.spans.iter().any(|s| s.3 == meta.id))
        };
        let example = hits()
            .find(|(_, e)| e.folder == meta.id)
            .or_else(|| hits().next())
            .cloned();
        docs.push(RuleDoc {
            meta,
            languages: vec![language],
            presets: presets.remove(meta.id).unwrap_or_default(),
            upstream: Vec::new(),
            example,
            same_why: Vec::new(),
            whys: vec![(
                meta.own_why().into_owned(),
                meta.description_text().into_owned(),
            )],
        });
    }
    for doc in &mut docs {
        let languages: Vec<&str> = doc.languages.iter().map(String::as_str).collect();
        doc.upstream = catalog::upstream_in(catalogs, doc.meta.id, &languages);
    }
    let order: Vec<&str> = std::iter::once("all")
        .chain(providers.iter().map(|p| p.language()))
        .collect();
    docs.sort_by_key(|d| {
        let rank = order.iter().position(|l| *l == d.languages[0]);
        (rank.unwrap_or(usize::MAX), d.meta.id)
    });
    docs
}

fn default_json(default: Opt) -> Value {
    match default {
        Opt::Bool(b) => json!(b),
        Opt::Int(i) => json!(i),
        Opt::Str(s) => json!(s),
        Opt::List(l) => json!(l),
    }
}

/// The text of `docs/ai/rules/rules.json`.
pub fn json(docs: &[RuleDoc]) -> String {
    let rules: Vec<Value> = docs.iter().map(entry).collect();
    serde_json::to_string_pretty(&json!({ "rules": rules })).expect("serializes") + "\n"
}

fn entry(d: &RuleDoc) -> Value {
    let m = d.meta;
    let options: Vec<Value> = m
        .options
        .iter()
        .map(|o| {
            let kind = match o.default {
                Opt::Bool(_) => "bool",
                Opt::Int(_) => "int",
                Opt::Str(_) => "string",
                Opt::List(_) => "list",
            };
            let mut v = json!({
                "name": o.name, "type": kind, "default": default_json(o.default),
                "description": o.description,
            });
            if let Check::OneOf(values) = o.check {
                v["choices"] = json!(values);
            }
            v
        })
        .collect();
    let mut v = json!({
        "id": m.id,
        "languages": d.languages,
        "category": m.category.as_str(),
        "default_level": m.default_level.as_str(),
        "fix": m.fix.map(|k| k.as_str()),
        "description": paragraphs(&m.description_text()),
        "options": options,
        "presets": d.presets,
        "upstream": d.upstream.iter().map(upstream).collect::<Vec<_>>(),
    });
    let why = m.why_text();
    if !why.is_empty() {
        v["why"] = json!(paragraphs(&why));
    }
    if let Some((language, e)) = &d.example {
        let spans: Vec<Value> = e
            .spans
            .iter()
            .filter(|s| s.3 == m.id)
            .map(|s| json!({ "line": s.0, "column": s.1, "length": s.2 }))
            .collect();
        // Lines where only other rules report: the page leaves them out of the shared input.
        let mut other: Vec<usize> = e
            .spans
            .iter()
            .filter(|s| s.3 != m.id && !e.spans.iter().any(|o| o.3 == m.id && o.0 == s.0))
            .map(|s| s.0)
            .collect();
        other.dedup();
        v["example"] = json!({
            "language": language, "input": e.text, "spans": spans, "other_lines": other,
            "fixed": e.fixed, "config": e.config,
        });
    }
    v
}

/// An upstream entry; the page and the note of a partial match only when there are some.
fn upstream(u: &Upstream) -> Value {
    let mut v = json!({ "tool": u.tool, "id": u.id, "partial": u.partial });
    for (key, text) in [("url", &u.url), ("note", &u.note)] {
        if !text.is_empty() {
            v[key] = json!(text);
        }
    }
    v
}

const UNDESCRIBED_HEADER: &str = "# Rules without a `why` text (docs/ai/development/rule-texts.md). Only shrinks: `cargo xtask codegen` drops a rule once it has one; a new rule needs `description` and `why`.\n";

/// The ratchet `docs/ai/rules/undescribed.txt`: the ids of `baseline` that still have no `why`
/// (all of them when there is no baseline yet). A rule without `why` that the baseline does not
/// list is an error.
pub fn undescribed(docs: &[RuleDoc], baseline: Option<&str>) -> anyhow::Result<String> {
    let listed: Option<std::collections::BTreeSet<&str>> = baseline.map(|b| {
        b.lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect()
    });
    if let Some(d) = docs.iter().find(|d| !d.same_why.is_empty()) {
        anyhow::bail!(
            "rule `{}` has the same `why` in `{}` and in {}: put it once in crates/core-rules/texts/why.txt",
            d.meta.id,
            d.languages[0],
            d.same_why.join(", ")
        );
    }
    let mut out = UNDESCRIBED_HEADER.to_string();
    let mut ids: Vec<&str> = docs
        .iter()
        .filter(|d| d.meta.why_text().trim().is_empty())
        .map(|d| d.meta.id)
        .collect();
    ids.sort();
    ids.dedup();
    for id in ids {
        match &listed {
            Some(l) if !l.contains(id) => anyhow::bail!(
                "rule `{id}` has no `why` text (a new rule needs `description` and `why`, see docs/ai/development/rule-texts.md)"
            ),
            _ => {
                out += id;
                out.push('\n');
            }
        }
    }
    Ok(out)
}

/// A text with its paragraphs (one per line of the metadata) as Markdown: a blank line between.
fn paragraphs(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// A table cell: one line, no pipes.
fn cell(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace('|', "\\|")
}

/// The rule table of `docs/ai/rules/builtin.md`.
pub fn table(docs: &[RuleDoc]) -> String {
    let mut out = String::from(
        "| Rule | Language | Category | Default | Fix | Description |\n|---|---|---|---|---|---|\n",
    );
    for d in docs {
        let m = d.meta;
        let mut description = cell(&m.summary());
        if !m.options.is_empty() {
            let options: Vec<String> = m
                .options
                .iter()
                .map(|o| {
                    let choices = match o.check {
                        Check::OneOf(values) => format!(
                            "; one of {}",
                            values
                                .iter()
                                .map(|v| format!("`{v}`"))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                        _ => String::new(),
                    };
                    format!(
                        "`{}` (default `{}`{choices}): {}",
                        o.name,
                        cell(&o.default.display()),
                        cell(o.description).trim_end_matches('.')
                    )
                })
                .collect();
            description += &format!(" Options: {}", options.join("; "));
            if !description.ends_with('.') {
                description.push('.');
            }
        }
        if !d.upstream.is_empty() {
            let upstream: Vec<String> = d.upstream.iter().map(|u| u.to_string()).collect();
            description += &format!(" Equivalent: {}.", upstream.join(", "));
        }
        out += &format!(
            "| `{}` | {} | {} | {} | {} | {description} |\n",
            m.id,
            d.languages.join(", "),
            m.category.as_str(),
            m.default_level.as_str(),
            m.fix.map_or("none", |k| k.as_str()),
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_rules::Category;
    use core_rules::rule::Level;

    static META: RuleMeta = RuleMeta {
        id: "demo",
        category: Category::Style,
        description: "Reports a demo.",
        why: "Because.",
        source: "",
        example: "",
        default_level: Level::Off,
        fix: None,
        requires: &[],
        uses: &[],
        options: &[],
    };

    #[test]
    fn the_same_why_in_two_languages_is_refused() {
        let doc = |same_why: Vec<String>| RuleDoc {
            meta: &META,
            languages: vec!["a".into(), "b".into()],
            presets: Vec::new(),
            upstream: Vec::new(),
            example: None,
            same_why,
            whys: Vec::new(),
        };
        assert!(undescribed(&[doc(Vec::new())], None).is_ok());
        let err = undescribed(&[doc(vec!["b".into()])], None).unwrap_err();
        assert!(err.to_string().contains("texts/why.txt"), "{err}");
    }
}
