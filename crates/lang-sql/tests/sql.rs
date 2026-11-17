//! Parse, round trip and the shared format checks on SQL probes and the bench inputs.

use core_syntax::{print, trivia};
use lang_sql::generated::kind;
use provider_api::format_testing::{Formatting, assert_file_names};
use provider_api::{LanguageProvider, SourceFile};

fn parse(text: &str, dialect: &str) -> provider_api::ParseResult {
    let mut file = SourceFile::new("a.sql", 1, text);
    file.language_version = Some(dialect.into());
    lang_sql::provider().parse(&file).unwrap()
}

/// Every check on one file in one dialect.
fn check(text: &str, dialect: &str) {
    let parsed = parse(text, dialect);
    assert!(
        parsed.errors.is_empty(),
        "{dialect}: {:?}",
        parsed.errors.first()
    );
    assert_eq!(print(&parsed.tree), text, "{dialect}: round trip");
    let grammar = lang_sql::provider().grammar().clone();
    trivia::check_attachment(&parsed.tree, |k| grammar.is_trivia(k))
        .unwrap_or_else(|e| panic!("{dialect}: {e}"));
    let unknown = (parsed.tree.descendants_with_tokens())
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == kind::UNKNOWN);
    assert!(unknown.is_none(), "{dialect}: unknown token {unknown:?}");
    // The shared subject parses by file name, which is ansi.
    if dialect == "ansi" {
        core_format::testing::assert_formats(&Formatting::new(lang_sql::provider(), "a.sql"), text);
    }
}

const PROBE: &str = "-- header
SELECT a, b AS c, /* inline */ count(*) -- trailing
FROM t1 JOIN t2 ON t1.id = t2.id
WHERE x IN (1, 2, 3) AND y = 'it''s'
GROUP BY a;

WITH cte AS (SELECT 1) SELECT * FROM cte;
INSERT INTO t (a) VALUES (1);
";

#[test]
fn probes_in_every_dialect() {
    for dialect in lang_sql::DIALECTS {
        check(PROBE, dialect);
    }
    check("", "ansi");
    check("\n\n", "ansi");
    check("-- only a comment", "ansi");
    check("SELECT [a b] FROM [t];", "tsql");
    check("SELECT `a` FROM `t` LIMIT 1;", "mysql");
    check("SELECT a::int, $1 FROM t;", "postgres");
}

#[test]
fn syntax_errors_are_reported_and_lossless() {
    for text in ["SELEC FROM WHERE ;;", "SELECT (1;", "select 'unterminated"] {
        let parsed = parse(text, "ansi");
        assert!(!parsed.errors.is_empty(), "{text:?}");
        assert_eq!(print(&parsed.tree), text);
    }
}

/// Deep nesting is an error or flat tokens, never a stack overflow (a 1 MB stack here).
#[test]
fn deep_nesting_does_not_overflow() {
    let n = 20_000;
    let inputs = [
        format!("SELECT {}1{};", "(".repeat(n), ")".repeat(n)),
        format!(
            "{}SELECT 1{};",
            "SELECT * FROM (".repeat(n),
            ") t".repeat(n)
        ),
        format!(
            "SELECT {}1{};",
            "CASE WHEN a THEN ".repeat(n),
            " END".repeat(n)
        ),
        format!("SELECT {}a;", "NOT ".repeat(n)),
        format!("SELECT {}a;", "- ".repeat(n)),
        format!("SELECT {}1{};", "ARRAY[".repeat(n), "]".repeat(n)),
        format!("SELECT {}a;", "f(".repeat(n)),
    ];
    std::thread::Builder::new()
        .stack_size(1 << 20)
        .spawn(move || {
            for text in &inputs {
                for dialect in lang_sql::DIALECTS {
                    assert_eq!(print(&lang_sql::parse(text, dialect).tree), *text);
                }
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn nodes_are_named_after_segment_types() {
    let parsed = parse("SELECT a FROM t WHERE a = 1;", "ansi");
    let kinds: Vec<_> = parsed.tree.descendants().map(|n| n.kind()).collect();
    for want in [
        kind::SELECT_STATEMENT,
        kind::SELECT_CLAUSE,
        kind::FROM_CLAUSE,
        kind::WHERE_CLAUSE,
        kind::OBJECT_REFERENCE,
    ] {
        assert!(kinds.contains(&want), "{want:?}");
    }
}

#[test]
fn bench_inputs() {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../bench/inputs/sql");
    for entry in std::fs::read_dir(dir).unwrap() {
        check(
            &std::fs::read_to_string(entry.unwrap().path()).unwrap(),
            "ansi",
        );
    }
}

#[test]
fn matches_the_extension() {
    assert_file_names(
        &lang_sql::provider(),
        &["a.sql", "dir/b.SQL"],
        &["a.sqlx", "sql"],
    );
}
