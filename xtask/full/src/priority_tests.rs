//! The priority table from fixture data: weights, the groups of the README table, mapped and unmapped causes, a blocked leg,
//! a leg with no differences.yaml (not measured), a tool support.yaml does not list and the New language lock.

use crate::priority::table;
use crate::queue::Tool as Parity;
use std::collections::BTreeMap;

const ROWS: &str = r"
- language: Alpha
  format: [{tool: fmt, name: Fmt}]
  lint: [{tool: lint, name: Lint}, {tool: pop, name: Pop, popular: true}]
- language: Beta
  format: [{tool: fixtures, name: OnlyFixtures}]
- language: Gamma
  format: [{tool: fmt, name: Held, blocked: waits on a decision, groups: [corpus-a]}]
";

const FMT: &str = r"
differences:
  big: gap  the large cause
  small: gap  the small cause
files:
  real-world: big:30, small:5
ratchet: {real-world: 60, tests: 1, corpus-a: 1}
target-files: {real-world: 100, tests: 1, corpus-a: 10}
";

const LINT: &str = r"
measured: {covered: 50, total: 100, main: abc123}
missed: {mapped-rule: 30, unmapped-rule: 15, rest: 5}
";

const POP: &str = "measured: {covered: 70, total: 100, main: abc123}\nmissed: {x: 30}\n";

const UNLISTED: &str = "measured: {covered: 10, total: 100, main: abc123}\nmissed: {y: 90}\n";

const CATALOG: &str = "tool: lint\nlanguages: [x]\nentries:\n  mapped-rule: rule some-rule\n  unmapped-rule: planned\n";

#[test]
fn table_from_fixture_data() {
    let rows = crate::support::rows(ROWS).unwrap();
    let parse = |t: &str| serde_saphyr::from_str::<Parity>(t).unwrap();
    let tools = BTreeMap::from([
        ("fmt".to_string(), parse(FMT)),
        ("lint".to_string(), parse(LINT)),
        ("pop".to_string(), parse(POP)),
        ("unlisted".to_string(), parse(UNLISTED)),
    ]);
    let catalogs = [languages::catalog::parse(CATALOG).unwrap()];
    insta::assert_snapshot!(table(&rows, &tools, &catalogs));
}

/// Weights 4, 1 and 0.25; the open lock lists the new language; secondary legs come last whatever the score.
#[test]
fn weights_new_language_and_order() {
    let parse = |t: &str| serde_saphyr::from_str::<Parity>(t).unwrap();
    let tools = BTreeMap::from([
        ("lint".to_string(), parse(LINT)),
        ("pop".to_string(), parse(POP)),
        (
            "ok".to_string(),
            parse("measured: {covered: 90, total: 100, main: abc123}\n"),
        ),
        ("unlisted".to_string(), parse(UNLISTED)),
    ]);
    let catalogs = [languages::catalog::parse(CATALOG).unwrap()];
    let rows = crate::support::rows(ROWS).unwrap();
    let locked = table(&rows, &tools, &catalogs);
    assert!(locked.contains("| 4 |") && locked.contains("| 1 |") && locked.contains("| 0.25 |"));
    assert!(!locked.contains("(new language)"));
    assert!(locked.contains("Score when open: 11.33"));
    let (primary, popular, other) = (
        locked.find("| 4 |"),
        locked.find("| 1 |"),
        locked.find("| 0.25 |"),
    );
    assert!(primary.is_some() && primary < popular && primary < other);
    let rows = crate::support::rows(
        "- language: Alpha\n  lint: [{tool: ok, name: Ok}, {tool: pop, name: Pop, popular: true}]\n",
    )
    .unwrap();
    let open = table(&rows, &tools, &catalogs);
    assert!(open.contains(
        "| (new language) | format + lint from 0 | 4 | 0% | 170.0 | - | estimate | 60 | 11.33 | - |"
    ));
    assert!(open.contains("New language: open"));
    assert!(open.find("(new language)") < open.find("| Alpha |"));
}
