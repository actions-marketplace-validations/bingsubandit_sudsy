//! New-language rows: the formula by hand, the best alternative, a paid parser, `~` and rejected values.

use crate::newlang::{demand, entries, plans, section};
use crate::support::rows;

/// Pushers and survey share that make demand exactly 0.8: sqrt(0.64) = 0.8 in both sources.
const GO: &str = r"
- language: Go
  gh: 5330774.4
  so: 40.192
  parser: ~1.0
  format: {tool: gofmt, gain: 0.1, cost: {layout: whitespace}, why: w}
  lint: {tool: lint, gain: 0.4, cost: {}, why: w}
";

fn one(yaml: &str, support: &str) -> anyhow::Result<Vec<crate::newlang::Entry>> {
    entries(yaml, &rows(support).unwrap())
}

fn err(yaml: &str, support: &str) -> String {
    one(yaml, support).unwrap_err().to_string()
}

/// 4 x 85 x 0.8 x 0.4 / (20 x (1 + 1)) = 2.72; format 272 x 0.1 / (20 x (1 + 0.7)) = 0.8; both 272 x 0.5 / (20 x 2.7) = 2.5185.
#[test]
fn formula_by_hand() {
    let list = one(GO, "").unwrap();
    let p = plans(&list[0]);
    let near = |a: f64, b: f64| assert!((a - b).abs() < 1e-9, "{a} != {b}");
    assert_eq!(
        p.iter().map(|p| p.name).collect::<Vec<_>>(),
        ["lint", "format", "both"]
    );
    near(p[0].score, 2.72);
    near(p[1].score, 0.8);
    near(p[2].score, 272.0 * 0.5 / (20.0 * 2.7));
    assert!(p.iter().all(|p| p.est), "the parser is an estimate");
}

/// The named parts of a cost: base 1.0, type worker +0.3, index +0.2, +0.1 per part, layout -0.3 / +0.2 / +0.5.
#[test]
fn cost_parts_by_hand() {
    let cost = |y: &str| {
        let yaml = format!(
            "- language: X\n  parser: 1.0\n  format: {{tool: t, gain: 0.1, cost: {y}, why: w}}\n"
        );
        one(&yaml, "").unwrap()[0]
            .format
            .as_ref()
            .unwrap()
            .cost
            .total()
    };
    let near = |a: f64, b: f64| assert!((a - b).abs() < 1e-9, "{a} != {b}");
    near(cost("{}"), 1.0);
    near(cost("{parts: 3}"), 1.3);
    near(cost("{layout: whitespace}"), 0.7);
    near(cost("{layout: width, parts: 2}"), 1.4);
    near(cost("{layout: penalty}"), 1.5);
    let lint = "- language: X\n  parser: 1.0\n  lint: {tool: t, gain: 0.1, cost: {types: true, index: true, parts: 1}, why: w}\n";
    near(
        one(lint, "").unwrap()[0]
            .lint
            .as_ref()
            .unwrap()
            .cost
            .total(),
        1.6,
    );
}

/// A leg is one tool: a list fails, a layout on a lint leg and a type worker on a format leg fail.
#[test]
fn one_primary_tool_per_leg() {
    let list = GO.replace(
        "format: {tool: gofmt, gain: 0.1, cost: {layout: whitespace}, why: w}",
        "format: [{tool: gofmt, gain: 0.1, cost: {}, why: w}, {tool: b, gain: 0.1, cost: {}, why: w}]",
    );
    assert!(
        err(&list, "").contains("one primary tool per leg"),
        "{}",
        err(&list, "")
    );
    let layout = GO.replace(
        "lint: {tool: lint, gain: 0.4, cost: {}",
        "lint: {tool: lint, gain: 0.4, cost: {layout: width}",
    );
    assert!(err(&layout, "").contains("lint leg has no layout"));
    let types = GO.replace("cost: {layout: whitespace}", "cost: {types: true}");
    assert!(err(&types, "").contains("format leg has no type worker"));
}

/// A retrofit entry scores the no-tool format leg of a supported language with the parser paid and a `retrofit` note;
/// its demand is the member of its row; it needs a row whose format leg has no reference tool.
#[test]
fn retrofit_rows() {
    let support = "- language: Docker\n  popularity: [{name: Docker, gh: 5330774.4, so: 40.192}]\n  stability: [d]\n  lint: [{tool: h, name: H}]\n";
    let yaml = "- language: Docker\n  retrofit: true\n  format: {tool: none found, gain: 1.0, cost: {parts: 1}, why: w}\n";
    let list = one(yaml, support).unwrap();
    let p = plans(&list[0]);
    // 272 x 1.0 / (20 x 1.1) with the parser paid
    assert_eq!(p.len(), 1);
    assert!((p[0].score - 272.0 / 22.0).abs() < 1e-9);
    let text = section(&list, &[]);
    assert!(text.contains("retrofit to no-reference-tool.md"), "{text}");
    assert!(
        err(
            yaml,
            "- language: Docker\n  popularity: [{name: Docker}]\n  format: [{tool: f, name: F}]\n"
        )
        .contains("is a retrofit")
    );
    assert!(
        err(
            &yaml
                .replace("Docker", "Elsewhere")
                .replace("  retrofit", "  parser: 1.0\n  retrofit"),
            ""
        )
        .contains("retrofit but not supported")
    );
    // without `retrofit` the same entry is refused: the leg exists
    assert!(err(&yaml.replace("  retrofit: true\n", ""), support).contains("fully supported"));
    // a member of a shared row takes its own popularity
    let shared = "- language: Row\n  popularity: [{name: A, gh: 5330774.4, so: 40.192}, {name: B}]\n  stability: [d]\n";
    let b = "- language: B\n  support: Row\n  retrofit: true\n  format: {tool: none found, gain: 1.0, cost: {}, why: w}\n";
    assert_eq!(one(b, shared).unwrap()[0].demand.v, 0.1);
}

/// A supported language with a missing leg has a paid parser (term 0); a deferred leg has no plan.
#[test]
fn paid_parser_and_deferred_leg() {
    let support = "- language: Ruby\n  popularity: [{name: Ruby, gh: 2082333.75, so: 15.7}]\n  lint: [{tool: rubocop, name: RuboCop}]\n";
    let yaml = r"
- language: Ruby
  format: {tool: rubocop, gain: 0.4, cost: {}, why: w}
";
    let list = one(yaml, support).unwrap();
    assert!(list[0].paid);
    // 340 x 0.5 x 0.4 / (20 x (0 + 1)) = 3.4
    let p = plans(&list[0]);
    assert_eq!(p.len(), 1);
    assert!((p[0].score - 3.4).abs() < 1e-9);
    assert!(section(&list, &[]).contains("| 0 (paid) |"));
    let deferred = format!("{yaml}  deferred: [format]\n");
    assert!(plans(&one(&deferred, support).unwrap()[0]).is_empty());
}

#[test]
fn bad_values_are_rejected() {
    let with = |from: &str, to: &str| GO.replace(from, to);
    assert!(err(&with("gain: 0.4", "gain: 0.5"), "").contains("gain 0.5"));
    assert!(err(&with("gh: 5330774.4", "gh: 9000000"), "").contains("gh Some(9000000.0)"));
    assert!(err(&with("so: 40.192", "so: 0"), "").contains("so Some(0.0)"));
    assert!(
        err(
            &with("cost: {layout: whitespace}, why: w}", "cost: {}, why: ''}"),
            ""
        )
        .contains("why are required")
    );
    assert!(
        err(&with("parser: ~1.0", "parser: ~1.0\n  speed: 1"), "")
            .contains("parsing languages.yaml")
    );
    assert!(err(&with("  parser: ~1.0\n", ""), "").contains("no `parser`"));
    let full = "- language: Go\n  format: [{tool: f, name: F}]\n  lint: [{tool: l, name: L}]\n";
    assert!(err(&GO.replace("  parser: ~1.0\n", ""), full).contains("fully supported"));
    let half = "- language: Go\n  lint: [{tool: l, name: L}]\n";
    assert!(err(&GO.replace("  parser: ~1.0\n", ""), half).contains("already has a lint leg"));
    assert!(err(GO, half).contains("parser is paid"));
    assert!(err(&format!("{GO}{GO}"), "").contains("twice"));
    // A supported language keeps `gh` and `so` in support.yaml.
    let format_only = GO
        .replace("  parser: ~1.0\n", "")
        .replace("  lint: {tool: lint, gain: 0.4, cost: {}, why: w}\n", "");
    assert!(err(&format_only, half).contains("`popularity` of its row"));
}

#[test]
fn estimates_print_a_tilde() {
    let yaml = GO
        .replace("gain: 0.4", "gain: ~0.4")
        .replace("  parser: ~1.0\n", "  parser: ~1.0\n  note: open\n");
    let list = one(&yaml, "").unwrap();
    let text = section(&list, &["Alpha lint X".into()]);
    assert!(
        text.contains("| 2.72~ | 0.80~ | 2.52~ | lint: lint lint |"),
        "{text}"
    );
    assert!(text.contains("Primary legs under the bar now: Alpha lint X."));
    assert!(text.contains("| open; estimate"), "{text}");
    assert!(text.contains("| Go | 0.80 | ~1 |"));
}

/// Per source `sqrt(x / top)`: a quarter of the top is 0.5. One source: the other is estimated by the median ratio 2.27
/// (the survey level over the pusher level), the demand is an estimate. Floor 0.1, cap 1.
#[test]
fn demand_scale_by_hand() {
    let near = |a: f64, b: f64| assert!((a - b).abs() < 1e-9, "{a} != {b}");
    let both = demand(Some(8_329_335.0 / 4.0), Some(62.8 / 4.0));
    near(both.v, 0.5);
    assert!(!both.est);
    // gh only: (0.5 + 0.5 x 2.27) / 2
    let gh = demand(Some(8_329_335.0 / 4.0), None);
    near(gh.v, 0.8175);
    assert!(gh.est);
    // so only: (0.5 / 2.27 + 0.5) / 2
    let so = demand(None, Some(62.8 / 4.0));
    near(so.v, (0.5 / 2.27 + 0.5) / 2.0);
    assert!(so.est);
    // 28,505 pushers and 608,541 pushers stay far apart (the log scale gave 0.57 and 0.84)
    let (a, b) = (
        demand(Some(28_505.0), Some(2.2)),
        demand(Some(608_541.0), Some(11.7)),
    );
    assert!(a.v < 0.15 && b.v > 0.3, "{a:?} {b:?}");
}

/// The mean under 0.1 is lifted to 0.1; a language in no source is 0.1 and an estimate; the top of one source caps at 1.
#[test]
fn demand_floor_and_cap() {
    assert_eq!(
        demand(Some(1_000.0), Some(0.1)),
        crate::newlang::V { v: 0.1, est: false }
    );
    assert_eq!(demand(None, None), crate::newlang::V { v: 0.1, est: true });
    assert_eq!(demand(Some(8_329_335.0), None).v, 1.0);
    let list = one(
        "- language: X\n  parser: 1.0\n  lint: {tool: t, gain: 0.1, cost: {}, why: w}\n",
        "",
    )
    .unwrap();
    assert_eq!(list[0].demand.v, 0.1);
}

/// Only the 25 best rows are table rows; the others are named with their score in one line, none dropped.
#[test]
fn table_shows_the_top_rows() {
    let yaml: String = (0..27)
        .map(|i| {
            format!(
                "- language: L{i}\n  parser: 1.0\n  lint: {{tool: t, gain: 0.1, cost: {{}}, why: w}}\n  gh: {}\n",
                100_000 * (i + 1)
            )
        })
        .collect();
    let text = section(&one(&yaml, "").unwrap(), &[]);
    assert_eq!(
        text.matches("\n| L").count(),
        26,
        "25 rows and the header: {text}"
    );
    assert!(text.contains("\n| L26 |"), "{text}");
    assert!(!text.contains("\n| L1 |"), "{text}");
    assert!(
        text.contains("The other 2 entries of the file, each with the score of its best plan: L1 "),
        "{text}"
    );
    assert!(text.contains("~; L0 "), "{text}");
}
