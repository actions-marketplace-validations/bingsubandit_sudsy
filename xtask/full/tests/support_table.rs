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
- language: Epsilon
  stability: [one, two]
  lint: [{tool: lint-a, name: LintA}]
- language: Zeta
  stability: [one]
  lint: [{tool: absent, name: Absent}]
";
    let table = xtask_full::support::render(&root, support, &|_| 7).unwrap();
    let (headline, ready) = (table.headline.clone(), table.ready.clone());
    let table = table.table;
    let rows: Vec<&str> = table.lines().skip(2).take(6).collect();
    assert_eq!(
        rows,
        [
            "| Alpha | ████████░░ 86% FmtA | ████████░░ 85% LintA | Ready |",
            // Only the first tool of each list decides the status: the unmeasured `Gone` does not.
            "| Beta | █████████░ 90% FmtA<br>not measured Gone | ████████░░ 85% LintA | Ready |",
            // No reference formatter: the stability share of the groups, no file count.
            "| Epsilon | █████████░ 90% stable · no reference tool | ████████░░ 85% LintA | Ready |",
            "| Gamma | none | ████████░░ 85% LintA | Lint only: Ready |",
            // An unmeasured lint tool is not a missing one: it does not pass.
            "| Zeta | █████████░ 90% stable · no reference tool | not measured Absent | In progress |",
            "| Delta | none | 7 rules · no reference tool | not measured |",
        ]
    );
    assert_eq!(headline, "Ready for 4 languages, early support for 2 more.");
    assert_eq!(
        ready,
        "Ready: Alpha, Beta, Epsilon and Gamma. The rest are in progress."
    );
}

/// A tool with a `note` gets a line under the table naming the tool, its language and the note.
#[test]
fn a_tool_note_is_printed_under_the_table() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/support");
    let support = "\n- language: Eta\n  format: [{tool: fmt-a, name: FmtA, note: wraps are not compared}]\n- language: Iota\n  format: [{tool: fmt-a, name: FmtB}]\n";
    let table = xtask_full::support::render(&root, support, &|_| 7).unwrap();
    assert!(
        table
            .table
            .contains("\nNotes: FmtA (Eta): wraps are not compared.\n"),
        "{}",
        table.table
    );
    let plain = xtask_full::support::render(
        &root,
        "\n- language: Iota\n  format: [{tool: fmt-a, name: FmtB}]\n",
        &|_| 7,
    )
    .unwrap();
    assert!(!plain.table.contains("Notes:"), "{}", plain.table);
}

/// A tool with a corpus group named in `support.yaml` shows `not measured` until that group has a
/// ratchet: the numbers of its other groups (a few local fixtures) never stand for the language.
#[test]
fn a_missing_corpus_ratchet_is_not_measured() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/support");
    let support = "\n- language: Theta\n  format: [{tool: fmt-a, name: FmtA, groups: [corpus]}]\n";
    let table = xtask_full::support::render(&root, support, &|_| 7).unwrap();
    assert!(
        table.table.contains("| Theta | not measured FmtA |"),
        "{}",
        table.table
    );
}

/// The local `fixtures` group never stands for a format leg: a tool whose only ratchet group is
/// `fixtures` shows `not measured`, with or without `groups` in `support.yaml`.
#[test]
fn a_fixtures_only_leg_is_not_measured() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/support");
    let support = "\n- language: Kappa\n  format: [{tool: fmt-fixtures, name: FmtF}]\n- language: Lambda\n  format: [{tool: fmt-fixtures, name: FmtG, groups: [fixtures]}]\n";
    let table = xtask_full::support::render(&root, support, &|_| 7).unwrap();
    for row in [
        "| Kappa | not measured FmtF |",
        "| Lambda | not measured FmtG |",
    ] {
        assert!(table.table.contains(row), "{}", table.table);
    }
}

/// Every format leg of the real `support.yaml` has a number from real code: the README states that
/// each number is measured on real-world code.
#[test]
fn no_format_leg_of_support_yaml_is_fixtures_only() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let support = std::fs::read_to_string(dir.join("support.yaml")).unwrap();
    let table = xtask_full::support::render(&dir.join("../.."), &support, &|_| 7).unwrap();
    let unmeasured: Vec<&str> = table
        .table
        .lines()
        .filter(|l| {
            l.split('|')
                .nth(2)
                .is_some_and(|f| f.contains("not measured"))
        })
        .collect();
    assert!(
        unmeasured.is_empty(),
        "format legs without a number: {unmeasured:?}"
    );
}

/// A leg that is not `blocked` and has no `target:` or `ratchet:` fails with the tool's name.
#[test]
fn a_leg_without_a_number_fails_unless_blocked() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/support");
    let check = |s| xtask_full::support::check_measured(&root, s);
    assert!(check("\n- language: A\n  format: [{tool: fmt-a, name: FmtA, groups: [real-world]}]\n  lint: [{tool: lint-a, name: LintA}]\n").is_ok());
    let e = check("\n- language: A\n  lint: [{tool: absent, name: Absent}]\n").unwrap_err();
    assert!(e.to_string().contains("`Absent`"), "{e}");
    let e = check("\n- language: A\n  format: [{tool: missing, name: Gone}]\n").unwrap_err();
    assert!(e.to_string().contains("`Gone`"), "{e}");
    assert!(
        check("\n- language: A\n  lint: [{tool: absent, name: Absent, blocked: waits}]\n").is_ok()
    );
}
