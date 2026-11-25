//! New-language rows: the formula by hand, the best alternative, a paid parser, `~` and rejected values.

use crate::newlang::{demand, entries, plans, section};
use crate::support::rows;

/// Pushers and survey share that make demand exactly 0.8: sqrt(0.64) = 0.8 in both sources.
const GO: &str = r"
- language: Go
  gh: 5330774.4
  so: 40.192
  parser: ~1.0
  format: [{tool: gofmt, gain: 0.1, reach: 0.95, cost: 0.6, why: w}]
  lint: [{tool: lint, gain: 0.4, reach: 0.85, cost: 1.0, why: w}]
";

fn one(yaml: &str, support: &str) -> anyhow::Result<Vec<crate::newlang::Entry>> {
    entries(yaml, &rows(support).unwrap())
}

fn err(yaml: &str, support: &str) -> String {
    one(yaml, support).unwrap_err().to_string()
}

/// 4 x 85 x 0.8 x 0.34 / (20 x (1 + 1)) = 2.312; format 272 x 0.095 / (20 x 1.6) = 0.8075; both 272 x 0.435 / (20 x 2.6) = 2.27538.
#[test]
fn formula_by_hand() {
    let list = one(GO, "").unwrap();
    let p = plans(&list[0]);
    let near = |a: f64, b: f64| assert!((a - b).abs() < 1e-9, "{a} != {b}");
    assert_eq!(
        p.iter().map(|p| p.name).collect::<Vec<_>>(),
        ["lint", "format", "both"]
    );
    near(p[0].score, 2.312);
    near(p[1].score, 0.8075);
    near(p[2].score, 2.275384615384615);
    assert!(p.iter().all(|p| p.est), "the parser is an estimate");
}

/// Of two alternatives of a leg the better score wins, whatever the order; reach under the bar is still scored and noted.
#[test]
fn best_alternative_wins() {
    let yaml = r"
- language: Rs
  gh: 8329335
  so: 62.8
  parser: 1.0
  lint:
    - {tool: no-types, gain: 0.7, reach: ~0.3, cost: 1.0, why: w}
    - {tool: types, gain: 0.7, reach: 0.85, cost: 3.0, why: w}
";
    let list = one(yaml, "").unwrap();
    let p = plans(&list[0]);
    // no types: 340 x 0.21 / (20 x 2) = 1.785; types: 340 x 0.595 / (20 x 4) = 2.529
    assert_eq!(p.len(), 1);
    assert_eq!(p[0].picks[0].1.tool, "types");
    assert!(!p[0].est);
    let cheap = yaml.replace("cost: 3.0", "cost: 9.0");
    let list = one(&cheap, "").unwrap();
    assert_eq!(plans(&list[0])[0].picks[0].1.tool, "no-types");
    assert!(plans(&list[0])[0].est);
    let text = section(&list, &[]);
    assert!(
        text.contains("cannot reach the bar: lint no-types reach ~0.3"),
        "{text}"
    );
    assert!(text.contains("estimate: pick only to measure it"), "{text}");
}

/// A supported language with a missing leg has a paid parser (term 0); a deferred leg has no plan.
#[test]
fn paid_parser_and_deferred_leg() {
    let support = "- language: Ruby\n  lint: [{tool: rubocop, name: RuboCop}]\n";
    let yaml = r"
- language: Ruby
  gh: 2082333.75
  so: 15.7
  format: [{tool: rubyfmt, gain: 0.4, reach: 0.9, cost: 1.0, why: w}]
";
    let list = one(yaml, support).unwrap();
    assert!(list[0].paid);
    // 340 x 0.5 x 0.36 / (20 x (0 + 1)) = 3.06
    let p = plans(&list[0]);
    assert_eq!(p.len(), 1);
    assert!((p[0].score - 3.06).abs() < 1e-9);
    assert!(section(&list, &[]).contains("| 0 (paid) |"));
    let deferred = format!("{yaml}  deferred: [format]\n");
    assert!(plans(&one(&deferred, support).unwrap()[0]).is_empty());
}

#[test]
fn bad_values_are_rejected() {
    let with = |from: &str, to: &str| GO.replace(from, to);
    assert!(err(&with("gain: 0.4", "gain: 0.5"), "").contains("gain 0.5"));
    assert!(err(&with("reach: 0.85", "reach: 1.5"), "").contains("reach 1.5"));
    assert!(err(&with("gh: 5330774.4", "gh: 9000000"), "").contains("gh Some(9000000.0)"));
    assert!(err(&with("so: 40.192", "so: 0"), "").contains("so Some(0.0)"));
    assert!(err(&with("why: w}]\n  lint", "why: ''}]\n  lint"), "").contains("why are required"));
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
}

#[test]
fn estimates_print_a_tilde() {
    let yaml = GO
        .replace("gain: 0.4", "gain: ~0.4")
        .replace("  parser: ~1.0\n", "  parser: ~1.0\n  note: open\n");
    let list = one(&yaml, "").unwrap();
    let text = section(&list, &["Alpha lint X".into()]);
    assert!(
        text.contains("| 2.31~ | 0.81~ | 2.28~ | lint: lint lint |"),
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
        "- language: X\n  parser: 1.0\n  lint: [{tool: t, gain: 0.1, reach: 1, cost: 1, why: w}]\n",
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
                "- language: L{i}\n  parser: 1.0\n  lint: [{{tool: t, gain: 0.1, reach: 1, cost: 1, why: w}}]\n  gh: {}\n",
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
