//! The parity catalog (`docs/ai/parity/*.yaml`) only names rules and options that exist.

use core_rules::{Extensions, SYNTAX_ERROR};
use languages::catalog::{self, Catalog, Entry};
use std::collections::BTreeMap;
use std::path::PathBuf;

const COVERAGE: [&str; 6] = [
    "covered", "partial", "missing", "sdk", "wontfix", "deferred",
];

fn parity_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/ai/parity")
}

fn catalogs() -> Vec<Catalog> {
    let mut out = Vec::new();
    for file in std::fs::read_dir(parity_dir()).unwrap() {
        let path = file.unwrap().path();
        if path.extension().is_some_and(|e| e == "yaml") {
            let text = std::fs::read_to_string(&path).unwrap();
            out.push(catalog::parse(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display())));
        }
    }
    out.sort_by(|a, b| a.tool.cmp(&b.tool));
    out
}

/// `status` names something that exists, or is one of the statuses that name nothing checkable.
fn check_status(
    catalog: &Catalog,
    entry: &Entry,
    rules: &[(String, &'static str)],
    options: &BTreeMap<String, Vec<String>>,
) -> Result<(), String> {
    let status = entry.sudsy.as_str();
    let (kind, value) = status.split_once(": ").unwrap_or((status, ""));
    match kind {
        "planned" | "deferred" | "formatter" if value.is_empty() => Ok(()),
        "sdk" | "config" | "wontfix" if !value.is_empty() => Ok(()),
        "rule" => entry.rules().try_for_each(|id| {
            if id.contains(' ') {
                return Err("a rule status names ids separated by `, `".into());
            }
            let known = id == SYNTAX_ERROR
                || rules.iter().any(|(language, known)| {
                    *known == id && (language == "all" || catalog.languages.contains(language))
                });
            known.then_some(()).ok_or_else(|| {
                format!(
                    "no rule `{id}` for {}; add the details to `notes`",
                    catalog.languages.join("/")
                )
            })
        }),
        "option" => {
            let parts: Vec<&str> = value.split('.').collect();
            let [language, "format", name] = parts[..] else {
                return Err(format!(
                    "`{value}` is not <language>.format.<name> (use `config:` for the rest)"
                ));
            };
            match options.get(language) {
                Some(names) if names.iter().any(|n| n == name) => Ok(()),
                Some(_) => Err(format!("no {language} option `{name}`")),
                None => Err(format!("no language `{language}`")),
            }
        }
        _ => Err(format!("unknown status `{status}`")),
    }
}

#[test]
fn every_rule_and_option_a_status_names_exists() {
    let providers = languages::providers();
    let rules: Vec<(String, &'static str)> = languages::rules(&providers, &Extensions::default())
        .into_iter()
        .map(|(language, rule)| (language, rule.meta().id))
        .collect();
    let options: BTreeMap<String, Vec<String>> = languages::schemas(&providers)
        .into_iter()
        .map(|(language, schema)| {
            let names = schema.defs().iter().map(|d| d.name.to_string()).collect();
            (language.to_string(), names)
        })
        .collect();
    let mut problems = Vec::new();
    for catalog in catalogs() {
        for entry in &catalog.entries {
            if !COVERAGE.contains(&entry.coverage.as_str()) {
                problems.push(format!(
                    "{} {}: unknown coverage `{}`",
                    catalog.tool, entry.id, entry.coverage
                ));
            }
            if let Err(e) = check_status(&catalog, entry, &rules, &options) {
                problems.push(format!("{} {}: {e}", catalog.tool, entry.id));
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn an_entry_may_name_several_rules() {
    let text = "tool: t\nlanguages: [js]\nrule-url: none\nentries:\n  a: rule x, y\n  b: rule z\n  c: planned\n";
    let c = catalog::parse(text).unwrap();
    let ids = |i: usize| c.entries[i].rules().collect::<Vec<_>>();
    assert_eq!(
        (ids(0), ids(1), ids(2)),
        (vec!["x", "y"], vec!["z"], vec![])
    );
}

#[test]
fn rule_pages_come_from_the_template_and_the_overrides() {
    let head = "tool: t\nlanguages: [js]\n";
    let body =
        "entries:\n  Lint/Syntax: rule x\n  opt: option js.format.line_width\n  b: planned\n";
    let url = |urls: &str| {
        let c = catalog::parse(&format!("{head}{urls}{body}")).unwrap();
        c.entries.iter().map(|e| e.url.clone()).collect::<Vec<_>>()
    };
    let t = "rule-url: \"https://x/{group}/{id-flat}/{name}/{id-lower}\"\n";
    assert_eq!(
        url(t),
        [
            "https://x/lint/lintsyntax/Syntax/lint/syntax",
            "",
            "https://x//b/b/b"
        ]
    );
    let o = "rule-urls:\n  b: https://y/b\n";
    assert_eq!(url(&format!("{t}{o}"))[2], "https://y/b");
    assert!(url("rule-url: none\n").iter().all(String::is_empty));
    assert!(catalog::parse(&format!("{head}rule-url: \"{{nope}}\"\n{body}")).is_err());
    assert!(
        catalog::parse(&format!(
            "{head}rule-url: none\nrule-urls:\n  zz: u\n{body}"
        ))
        .is_err()
    );
    assert!(
        catalog::parse(&format!("{head}{body}")).is_err(),
        "a catalog names its rule-url"
    );
}
