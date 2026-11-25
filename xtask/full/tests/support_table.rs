//! The README "Language support" table: bars, status and `not measured` cells from fixture ratchets.

#[test]
fn table_from_fixture_ratchets() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/support");
    // Popularity (1.0 = top of both sources): Gamma 1, Alpha 0.5, Beta 0.25, the rest none (0.1, an estimate).
    let support = r#"
- language: Alpha
  popularity: [{name: A, gh: 2082333.75, so: 15.7}]
  format: [{tool: fmt-a, name: FmtA, groups: [real-world, tests]}]
  lint: [{tool: lint-a, name: LintA}]
- language: Beta
  popularity: [{name: B, gh: 520583.4375, so: 3.925}]
  format: [{tool: fmt-a, name: FmtA, groups: [real-world]}, {tool: missing, name: Gone}]
  lint: [{tool: lint-a, name: LintA}]
- language: Gamma
  popularity: [{name: G, gh: 8329335, so: 62.8}]
  lint: [{tool: lint-a, name: LintA}]
- language: Delta
- language: Epsilon
  exempt: true
  stability: [one, two]
  lint: [{tool: lint-a, name: LintA}]
- language: Zeta
  popularity: [{name: Z, gh: 2082333.75, so: 15.7}]
  exempt: true
  stability: [one]
  lint: [{tool: absent, name: Absent}]
"#;
    let table = xtask_full::support::render(&root, support, &|_| 7).unwrap();
    let (headline, ready) = (table.headline.clone(), table.ready.clone());
    let table = table.table;
    let rows: Vec<&str> = table.lines().skip(2).take(6).collect();
    assert_eq!(
        rows,
        [
            // Ready rows first, by popularity; Gamma (lint only) is the most popular.
            "| Gamma | none | ████████░░ 85% LintA | Lint only: Ready |",
            "| Alpha | ████████░░ 86% FmtA | ████████░░ 85% LintA | Ready |",
            // Only the first tool of each list decides the status: the unmeasured `Gone` does not.
            "| Beta | █████████░ 90% FmtA<br>not measured Gone | ████████░░ 85% LintA | Ready |",
            // No reference formatter: the stability share of the groups, no file count.
            "| Epsilon | █████████░ 90% stable · no reference tool | ████████░░ 85% LintA | Ready |",
            // An unmeasured lint tool is not a missing one: it does not pass.
            // The others follow, by popularity.
            "| Zeta | █████████░ 90% stable · no reference tool | not measured Absent | In progress |",
            "| Delta | none | 7 rules · no reference tool | not measured |",
        ]
    );
    assert_eq!(headline, "Ready for 4 languages, early support for 2 more.");
    assert_eq!(
        ready,
        "Ready: Gamma, Alpha, Beta and Epsilon. The rest are in progress."
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
    let check = |s: &str| {
        xtask_full::support::check_measured(
            &root,
            &s.replace(
                "- language: A\n",
                "- language: A\n  popularity: [{name: A}]\n",
            ),
        )
    };
    assert!(check("\n- language: A\n  format: [{tool: fmt-a, name: FmtA, groups: [real-world]}]\n  lint: [{tool: lint-a, name: LintA}]\n").is_ok());
    let e = check("\n- language: A\n  lint: [{tool: absent, name: Absent}]\n").unwrap_err();
    assert!(e.to_string().contains("`Absent`"), "{e}");
    let e = check("\n- language: A\n  format: [{tool: missing, name: Gone}]\n").unwrap_err();
    assert!(e.to_string().contains("`Gone`"), "{e}");
    assert!(
        check("\n- language: A\n  lint: [{tool: absent, name: Absent, blocked: waits}]\n").is_ok()
    );
}

/// A new language's leg with no reference tool is Ready at stable 100% and agreement 80% (format) and checklist 85% with
/// precision 90% (lint); one number under its bar, or no evidence, keeps it In progress. An `exempt` leg needs none.
#[test]
fn a_leg_with_no_reference_tool_needs_its_numbers() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/support");
    let support = r#"
- language: Mu
  popularity: [{name: M, gh: 2082333.75, so: 15.7}]
  stability: [full]
  evidence: evidence-ok
- language: Nu
  stability: [full]
  evidence: evidence-low
- language: Xi
  stability: [one]
  evidence: evidence-ok
- language: Omicron
  exempt: true
  stability: [full]
- language: Pi
  format: [{tool: fmt-a, name: FmtA, groups: [real-world]}]
  evidence: evidence-ok
- language: Rho
  format: [{tool: fmt-a, name: FmtA, groups: [real-world]}]
  evidence: evidence-low
"#;
    let table = xtask_full::support::render(&root, support, &|_| 5)
        .unwrap()
        .table;
    let row = |name: &str| {
        table
            .lines()
            .find(|l| l.starts_with(&format!("| {name} |")))
            .unwrap()
            .to_string()
    };
    assert_eq!(
        row("Mu"),
        "| Mu | ██████████ 100% stable · ████████░░ 80% agreement · no reference tool | 5 rules · ████████░░ 85% checklist · █████████░ 90% precision · no reference tool | Ready |"
    );
    // Agreement 79%, stable under 100%, a lint precision of 89%: each alone keeps the row In progress.
    for name in ["Nu", "Xi", "Rho"] {
        assert!(row(name).ends_with("| In progress |"), "{}", row(name));
    }
    assert!(row("Pi").ends_with("| Ready |"), "{}", row("Pi"));
    assert!(row("Omicron").ends_with("| Ready |"), "{}", row("Omicron"));
    // No evidence for a non-exempt leg with no tool fails the check; the exempt one passes.
    let missing = xtask_full::support::check_measured(
        &root,
        "- language: Q\n  popularity: [{name: Q}]\n  stability: [full]\n",
    )
    .unwrap_err();
    assert!(
        missing.to_string().contains("needs `evidence:`"),
        "{missing}"
    );
    xtask_full::support::check_measured(
        &root,
        "- language: Q\n  popularity: [{name: Q}]\n  exempt: true\n  stability: [full]\n",
    )
    .unwrap();
}
