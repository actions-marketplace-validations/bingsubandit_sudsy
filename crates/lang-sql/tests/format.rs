//! Every expectation passes the shared checks (lossless, meaning, comments, idempotency).

use core_format::DirectiveTable;
use core_format::testing::assert_formats;
use provider_api::format::format_file;
use provider_api::format_testing::Formatting;
use provider_api::{LanguageProvider, SourceFile};

#[test]
fn lays_out_lists_and_keeps_comments() {
    let input = "-- header\nSELECT /* inline */ a,b -- trailing\nFROM t;\n/* block\n   comment */\nSELECT 1;\n";
    let subject = Formatting::new(lang_sql::provider(), "a.sql");
    let want = "-- header\nSELECT /* inline */\n    a,\n    b -- trailing\nFROM t;\n/* block\n   comment */\nSELECT 1;\n";
    assert_eq!(assert_formats(&subject, input), want);
}

fn formats(input: &str) -> String {
    assert_formats(&Formatting::new(lang_sql::provider(), "a.sql"), input)
}

#[test]
fn long_lines_break_like_sqlfluff() {
    let call = "select a from t where x = sch.fname(arg_number_one, arg_number_two, arg_number_three, arg_number_four);\n";
    assert_eq!(
        formats(call),
        "select a\nfrom t\nwhere\n    x\n    = sch.fname(\n        arg_number_one, arg_number_two, arg_number_three, arg_number_four\n    );\n"
    );
    let list = "select * from table_one, table_two, table_three, table_four, table_five, table_six, table_seven, table_eight, table_nine;\n";
    assert_eq!(
        formats(list),
        "select *\nfrom table_one, table_two, table_three, table_four, table_five, table_six,\n    table_seven, table_eight, table_nine;\n"
    );
}

#[test]
fn a_trailing_comment_over_the_width_moves_above() {
    let input = "select a from t where b = 1 and c = 2 and d = 3 and e = 4 -- a long comment that does not fit on the line\n;\n";
    assert_eq!(
        formats(input),
        "-- a long comment that does not fit on the line\nselect a from t\nwhere b = 1 and c = 2 and d = 3 and e = 4\n;\n"
    );
}

#[test]
fn capitalisation_follows_the_first_word_or_the_option() {
    let input = "select A, b from T where X = Null and y is NULL and z = CAST(1 AS Int);\n";
    assert_eq!(
        formats(input),
        "select\n    A,\n    B\nfrom T\nwhere X = Null and Y is Null and Z = CAST(1 as Int);\n"
    );
    let mut subject = Formatting::new(lang_sql::provider(), "a.sql");
    for (name, value) in [("keyword_case", "upper"), ("literal_case", "lower")] {
        subject
            .options
            .push((name, core_config::OptionValue::String(value.into())));
    }
    assert_eq!(
        assert_formats(&subject, input),
        "SELECT\n    A,\n    B\nFROM T\nWHERE X = null AND Y IS null AND Z = CAST(1 AS Int);\n"
    );
}

#[test]
fn leading_commas() {
    let mut subject = Formatting::new(lang_sql::provider(), "a.sql");
    subject.options.push((
        "comma_position",
        core_config::OptionValue::String("leading".into()),
    ));
    assert_eq!(
        assert_formats(
            &subject,
            "select a,\n b\n, c from t where x in (1,\n 2\n,3)\n"
        ),
        "select\n    a\n    , b\n    , c\nfrom t\nwhere x in (\n    1\n    , 2\n    , 3\n)\n"
    );
}

#[test]
fn spacing_of_names_brackets_and_prefix_operators() {
    assert_eq!(
        formats("select max (a) from t;\n"),
        "select max(a) from t;\n"
    );
    assert_eq!(
        formats("select ~ 1, ~ ~ 2;\n"),
        "select\n    ~1,\n    ~ ~2;\n"
    );
    assert_eq!(
        formats("with c as\n(select 1)\nselect * from c;\n"),
        "with c as (select 1)\n\nselect * from c;\n"
    );
}

#[test]
fn function_signature_brackets_close_up_and_procedure_brackets_stay() {
    assert_eq!(
        formats("create function f (a int) returns int as $$ select 1 $$ language sql;\n"),
        "create function f(a int) returns int as $$ select 1 $$ language sql;\n"
    );
    assert_eq!(
        formats("create procedure p (a int) language sql as $$ select 1 $$;\n"),
        "create procedure p (a int) language sql as $$ select 1 $$;\n"
    );
}

#[test]
fn a_line_start_colon_command_keeps_its_blank() {
    let subject = Formatting::new(lang_sql::provider(), "a.sql");
    assert_eq!(
        assert_formats(&subject, ":r file.sql\nselect 1;\n"),
        ":r file.sql\nselect 1;\n"
    );
}

#[test]
fn errors_are_refused() {
    let provider = lang_sql::provider();
    let file = SourceFile::new("a.sql", 1, "SELEC FROM WHERE ;;");
    let options = provider.option_schema().defaults();
    assert!(format_file(&provider, &file, &options, &DirectiveTable::default()).is_err());
}

#[test]
fn tsql_indents_if_else_and_while_bodies() {
    let mut subject = Formatting::new(lang_sql::provider(), "a.sql");
    subject
        .options
        .push(("dialect", core_config::OptionValue::String("tsql".into())));
    assert_eq!(
        assert_formats(
            &subject,
            "IF @a = 1\nSELECT 1\nELSE\nSELECT 2;\nWHILE @a < 3\nSET @a = @a + 1;\n"
        ),
        "IF @a = 1\n    SELECT 1\nELSE\n    SELECT 2;\nWHILE @a < 3\n    SET @a = @a + 1;\n"
    );
}
