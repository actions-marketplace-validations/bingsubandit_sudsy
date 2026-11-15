//! The formatter (sqruff's fixer with sqlfluff's layout and capitalisation rules): every
//! expectation passes the shared checks (lossless, meaning, comments, idempotency) in ansi, and
//! the dialects and the fallbacks go through `format_file`.

use core_config::OptionValue;
use core_format::DirectiveTable;
use core_format::testing::assert_formats;
use provider_api::format::format_file;
use provider_api::format_testing::Formatting;
use provider_api::{LanguageProvider, SourceFile};

fn s(v: &str) -> OptionValue {
    OptionValue::String(v.into())
}

fn fmt(input: &str) -> String {
    assert_formats(&Formatting::new(lang_sql::provider(), "a.sql"), input)
}

fn fmt_with(options: Vec<(&'static str, OptionValue)>, input: &str) -> String {
    let mut subject = Formatting::new(lang_sql::provider(), "a.sql");
    subject.options = options;
    assert_formats(&subject, input)
}

/// `input` formatted as `dialect` twice (the second run must change nothing).
fn fmt_in(dialect: &str, input: &str) -> String {
    let provider = lang_sql::provider();
    let mut options = provider.option_schema().defaults();
    options.set("dialect", s(dialect));
    let run = |text: &str| {
        let mut file = SourceFile::new("a.sql", 1, text);
        file.language_version = Some(dialect.into());
        format_file(&provider, &file, &options, &DirectiveTable::default()).unwrap()
    };
    let once = run(input);
    assert_eq!(run(&once), once, "{dialect}: idempotent");
    once
}

#[test]
fn layout() {
    assert_eq!(
        fmt(
            "select a,b,   c from t1 join t2 on t1.id=t2.id where x in (1,2) and y='it''s' group by a order by 1 -- last\n;\n\n\n\nINSERT INTO t (a) VALUES (1)\n"
        ),
        "-- last\nselect\n    a,\n    b,\n    c\nfrom t1 join t2 on t1.id = t2.id\nwhere x in (1, 2) and y = 'it''s'\ngroup by a\norder by 1\n;\n\n\ninsert into t (a) values (1)\n"
    );
}

#[test]
fn comments_stay() {
    let input = "-- header\nSELECT /* inline */ a -- trailing\nFROM t;\n/* block\n   comment */\nSELECT 1;\n";
    assert_eq!(fmt(input), input);
}

#[test]
fn capitalisation_follows_the_first_spelling_or_the_option() {
    assert_eq!(fmt("select A from T\n"), "select A from T\n");
    assert_eq!(
        fmt("SELECT a, Count(*) from T\n"),
        "SELECT\n    a,\n    Count(*)\nFROM t\n"
    );
    let upper = vec![
        ("keyword_case", s("upper")),
        ("identifier_case", s("upper")),
        ("function_case", s("lower")),
        ("literal_case", s("lower")),
        ("type_case", s("upper")),
    ];
    assert_eq!(
        fmt_with(
            upper,
            "select a, Count(*) from t where b is NULL and c = cast(d as int)\n"
        ),
        "SELECT\n    A,\n    count(*)\nFROM T\nWHERE B IS null AND C = cast(D AS INT)\n"
    );
    let preserve = vec![
        ("keyword_case", s("preserve")),
        ("identifier_case", s("preserve")),
    ];
    assert_eq!(
        fmt_with(preserve, "Select Ab from T where X = 1\n"),
        "Select Ab from T\nwhere X = 1\n"
    );
}

#[test]
fn layout_options() {
    let leading = vec![("comma_position", s("leading"))];
    assert_eq!(
        fmt_with(leading, "select a, b from t\n"),
        "select\n    a\n    , b\nfrom t\n"
    );
    let tab = vec![("indent_style", s("tab"))];
    assert_eq!(
        fmt_with(tab, "select a, b from t\n"),
        "select\n\ta,\n\tb\nfrom t\n"
    );
    let narrow = vec![
        ("line_width", OptionValue::Int(20)),
        ("indent_size", OptionValue::Int(2)),
    ];
    assert_eq!(
        fmt_with(
            narrow,
            "select a from t where aaaa = 1 and bbbb = 2 and cccc = 3\n"
        ),
        "select a\nfrom t\nwhere\n  aaaa = 1\n  and bbbb = 2\n  and cccc = 3\n"
    );
}

#[test]
fn dialects() {
    assert_eq!(
        fmt_in("postgres", "select a::int,b from t\n"),
        "select\n    a::int,\n    b\nfrom t\n"
    );
    assert_eq!(
        fmt_in("mysql", "SELECT `a`,`b` FROM `t` LIMIT 1\n"),
        "SELECT\n    `a`,\n    `b`\nFROM `t` LIMIT 1\n"
    );
    assert_eq!(
        fmt_in("tsql", "SELECT [a],[b] FROM [t]\n"),
        "SELECT\n    [a],\n    [b]\nFROM [t]\n"
    );
}

#[test]
fn line_endings_and_final_newline() {
    assert_eq!(
        fmt_in("ansi", "select a\r\nfrom t"),
        "select a\r\nfrom t\r\n"
    );
}

#[test]
fn a_fix_that_changes_the_code_is_dropped() {
    // Postgres nests block comments; sqruff's fixer moves text into this one.
    let input = "SELECT /* x\n/* y\n*/ z\n*/\n'a' AS s;\n";
    assert_eq!(fmt_in("postgres", input), input);
    // sqruff mangles `~ ~ ~4`: that statement stays, the others are formatted.
    assert_eq!(
        fmt("select a,b from t;\n\nSELECT 8 | ~ ~ ~4; -- x\nselect c from u;\n"),
        "select\n    a,\n    b\nfrom t;\n\nSELECT 8 | ~ ~ ~4; -- x\nselect c from u;\n"
    );
}

#[test]
fn errors_are_refused() {
    let provider = lang_sql::provider();
    let file = SourceFile::new("a.sql", 1, "SELEC FROM WHERE ;;");
    let options = provider.option_schema().defaults();
    assert!(format_file(&provider, &file, &options, &DirectiveTable::default()).is_err());
}
