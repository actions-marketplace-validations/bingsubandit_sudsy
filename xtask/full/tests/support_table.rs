//! The README "Language support" table: bars, status and `not measured` cells from fixture ratchets.

#[test]
fn table_from_fixture_ratchets() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/support");
    let support = r"
- language: Alpha
  format: [{tool: fmt-a, name: FmtA, groups: [real-world, tests]}]
  lint: [{tool: lint-a, name: LintA}]
- language: Beta
  format: [{tool: fmt-a, name: FmtA, groups: [real-world]}, {tool: missing, name: Gone}]
  lint: [{tool: lint-a, name: LintA}]
- language: Gamma
  lint: [{tool: lint-a, name: LintA}]
- language: Delta
";
    let table = xtask_full::support::render(&root, support, &|_| 0).unwrap();
    let (headline, ready) = (table.headline.clone(), table.ready.clone());
    let table = table.table;
    let rows: Vec<&str> = table.lines().skip(2).take(4).collect();
    assert_eq!(
        rows,
        [
            "| Alpha | ████████░░ 86% FmtA | ████████░░ 85% LintA | Ready |",
            "| Gamma | none | ████████░░ 85% LintA | Lint only: Ready |",
            "| Beta | █████████░ 90% FmtA<br>not measured Gone | ████████░░ 85% LintA | In progress |",
            "| Delta | none | none | not measured |",
        ]
    );
    assert_eq!(headline, "Ready for 2 languages, early support for 2 more.");
    assert_eq!(ready, "Ready: Alpha and Gamma. The rest are in progress.");
}
