//! The priority table from fixture data: weights, real-world groups only, mapped and unmapped causes, a blocked leg,
//! a leg with fixtures only (not measured) and the New language lock.

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
  format: [{tool: fmt, name: Held, blocked: waits on a decision, corpus: [corpus-a]}]
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

const FIXTURES: &str = "ratchet: {fixtures: 9}\ntarget-files: {fixtures: 9}\n";

const LINT: &str = r"
measured: {covered: 50, total: 100, main: abc123}
missed: {mapped-rule: 30, unmapped-rule: 15, rest: 5}
";

const POP: &str = "measured: {covered: 70, total: 100, main: abc123}\nmissed: {x: 30}\n";

const CATALOG: &str = "tool: lint\nlanguages: [x]\nentries:\n  mapped-rule: rule some-rule\n  unmapped-rule: planned\n";

#[test]
fn table_from_fixture_data() {
    let rows = crate::support::rows(ROWS).unwrap();
    let parse = |t: &str| serde_saphyr::from_str::<Parity>(t).unwrap();
    let tools = BTreeMap::from([
        ("fmt".to_string(), parse(FMT)),
        ("fixtures".to_string(), parse(FIXTURES)),
        ("lint".to_string(), parse(LINT)),
        ("pop".to_string(), parse(POP)),
    ]);
    let catalogs = [languages::catalog::parse(CATALOG).unwrap()];
    insta::assert_snapshot!(table(&rows, &tools, &catalogs));
}
