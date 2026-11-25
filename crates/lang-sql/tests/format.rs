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

fn formats_as(dialect: &str, input: &str) -> String {
    let mut subject = Formatting::new(lang_sql::provider(), "a.sql");
    subject
        .options
        .push(("dialect", core_config::OptionValue::String(dialect.into())));
    assert_formats(&subject, input)
}

#[test]
fn tsql_end_closes_a_block_but_not_a_case() {
    let input = "BEGIN\nSELECT CASE WHEN @a = 1 THEN 1 ELSE 2 END AS x\nBEGIN\nSELECT 3\nEND\nSELECT 4\nEND\nSELECT 5\n";
    assert_eq!(
        formats_as("tsql", input),
        "BEGIN\n    SELECT CASE WHEN @a = 1 THEN 1 ELSE 2 END AS x\n    BEGIN\n        SELECT 3\n    END\n    SELECT 4\nEND\nSELECT 5\n"
    );
}

#[test]
fn join_hint_words_stay_names_when_no_join_follows() {
    for dialect in ["tsql", "postgres", "mysql"] {
        assert_eq!(
            formats_as(dialect, "select * from t hash where hash.a = 1;\n"),
            "select * from t hash\nwhere hash.a = 1;\n",
            "{dialect}"
        );
        assert_eq!(
            formats_as(dialect, "select * from t as loop join u on loop.a = u.a;\n"),
            "select * from t as loop join u on loop.a = u.a;\n",
            "{dialect}"
        );
    }
}

#[test]
fn copy_rows_without_a_terminator_are_not_data() {
    let input = "copy t from stdin;\nselect 1;\n";
    assert_eq!(formats_as("postgres", input), input);
}

#[test]
fn mysql_delimiter_may_be_a_word() {
    let input = "DELIMITER d//\nSELECT 1 d//\nDELIMITER ;\n";
    assert_eq!(formats_as("mysql", input), input);
    // The rest of the `DELIMITER` line is ignored by the client and kept apart from the delimiter.
    let ignored = "SELECT 30;\nDELIMITER // ;\nSELECT 31 //\n";
    assert_eq!(
        formats_as("mysql", ignored),
        "SELECT 30;\nDELIMITER // ;\nSELECT 31//\n"
    );
}

#[test]
fn tsql_else_if_chains_stay_at_the_level_of_the_first_if() {
    assert_eq!(
        formats_as(
            "tsql",
            "IF @a = 1\nSELECT 1\nELSE IF @a = 2\nBEGIN\nSELECT 2\nEND\nELSE\nIF @a = 3\nSELECT 3\n"
        ),
        "IF @a = 1\n    SELECT 1\nELSE IF @a = 2\n    BEGIN\n        SELECT 2\n    END\nELSE\nIF @a = 3\n    SELECT 3\n"
    );
}

#[test]
fn tsql_drop_and_create_table_end_at_the_next_statement() {
    assert_eq!(
        formats_as(
            "tsql",
            "IF 1 = 1\nDROP TABLE #E\nCREATE TABLE t (a int) WITH (x = 1)\nWHILE 1 = 1\nBEGIN\nSELECT 1\nEND\n"
        ),
        "IF 1 = 1\n    DROP TABLE #E\nCREATE TABLE t (a int) WITH (x = 1)\nWHILE 1 = 1\n    BEGIN\n        SELECT 1\n    END\n"
    );
}

#[test]
fn tsql_select_into_and_create_index_end_at_the_next_statement() {
    assert_eq!(
        formats_as(
            "tsql",
            "SELECT a\nINTO #f\nFROM t\nJOIN u\nON t.a = u.a\nCREATE INDEX i ON #f (a)\nIF 1 = 1\nDROP TABLE #f\nSELECT 2\n"
        ),
        "SELECT a\nINTO #f\nFROM t\nJOIN u\n    ON t.a = u.a\nCREATE INDEX i ON #f (a)\nIF 1 = 1\n    DROP TABLE #f\nSELECT 2\n"
    );
}

#[test]
fn tsql_update_of_a_table_variable_ends_at_the_next_statement() {
    assert_eq!(
        formats_as(
            "tsql",
            "UPDATE @M SET a = 1\nFROM @M\nWHERE b = 1\nSELECT (\n1)\n"
        ),
        "UPDATE @M SET a = 1\nFROM @M\nWHERE b = 1\nSELECT (\n    1\n)\n"
    );
}

#[test]
fn tsql_cross_apply_takes_a_built_in_table_function() {
    assert_eq!(
        formats_as(
            "tsql",
            "SELECT 1 FROM t CROSS APPLY ::fn_x(t .id) AS e JOIN u ON 1 = 1\n"
        ),
        "SELECT 1 FROM t CROSS APPLY ::fn_x(t.id) AS e JOIN u ON 1 = 1\n"
    );
}

#[test]
fn tsql_alter_table_ends_at_the_next_statement_and_a_computed_column_is_an_expression() {
    assert_eq!(
        formats_as(
            "tsql",
            "ALTER TABLE t ADD c AS (CASE WHEN a = 1 THEN 2 ELSE 3 END)\nCREATE INDEX i ON t (c)\n"
        ),
        "ALTER TABLE t ADD c AS (CASE WHEN a = 1 THEN 2 ELSE 3 END)\nCREATE INDEX i ON t (c)\n"
    );
}

#[test]
fn tsql_declare_breaks_before_the_equals_of_a_long_value() {
    assert_eq!(
        formats_as(
            "tsql",
            "DECLARE @EditionOfTheServer NVARCHAR(MAX) = CAST(SERVERPROPERTY('Edition') AS NVARCHAR(128)), @e int;\n"
        ),
        "DECLARE\n    @EditionOfTheServer NVARCHAR(MAX)\n    = CAST(SERVERPROPERTY('Edition') AS NVARCHAR(128)),\n    @e INT;\n"
    );
}

#[test]
fn tsql_go_ends_a_set_option_and_end_semicolon_may_precede_else() {
    assert_eq!(
        formats_as("tsql", "SET ANSI_NULLS ON\nGO\nBEGIN\nSELECT 1\nEND\n"),
        "SET ANSI_NULLS ON\nGO\nBEGIN\n    SELECT 1\nEND\n"
    );
    let input = "IF 1 = 1 BEGIN SELECT 1 END; ELSE BEGIN SELECT 2 END\n";
    assert_eq!(formats_as("tsql", input), input);
}

#[test]
fn a_lifted_comment_leaves_a_blank_between_its_neighbours() {
    let input = "select\n    aaaaaaaaaaaaaaaaaaaaaaaaaaaaa + 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb' + -- c\n    case when x then 1 end;\n";
    assert!(formats(input).contains("\n    + case when x then 1 end;"));
}

#[test]
fn an_operator_never_fuses_with_a_sigil_after_it() {
    // Postgres has `<@`; elsewhere `@y` is a variable (T-SQL, MySQL) or an operator and a name.
    for dialect in ["ansi", "postgres", "mysql", "tsql"] {
        assert_eq!(
            formats_as(dialect, "select a from t where x < @y;\n"),
            "select a from t\nwhere x < @y;\n",
            "{dialect}"
        );
    }
    assert_eq!(
        formats_as("mysql", "select a from t where x<@y;\n"),
        "select a from t\nwhere x < @y;\n"
    );
}

#[test]
fn mysql_first_body_statement_is_a_statement_of_its_own() {
    let input = "create procedure p() begin\nif aaaaaaaaaaaaaaaaaaaaaa = 1 and bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb = 2 and ccccccccccccccccccccc = 3 then select 1; end if; end;\n";
    assert!(formats_as("mysql", input).contains(
        "\nif\n    aaaaaaaaaaaaaaaaaaaaaa = 1\n    and bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb = 2\n"
    ));
}

#[test]
fn a_keyword_word_before_an_arrow_is_a_column_and_types_are_names_after_references() {
    let input = "CREATE TABLE t (a INT GENERATED ALWAYS AS (value ->> 'x') STORED, b INT REFERENCES types (id));\n";
    for dialect in ["postgres", "mysql"] {
        let out = formats_as(dialect, input);
        assert!(out.contains("(value ->> 'x')"), "{dialect}: {out}");
        assert!(out.contains("REFERENCES types (id)"), "{dialect}: {out}");
    }
}

#[test]
fn tsql_calls_a_procedure_without_exec() {
    let input = "sp_configure 'show advanced options', 1\nRECONFIGURE WITH OVERRIDE\nGO\n";
    assert_eq!(formats_as("tsql", input), input);
}

#[test]
fn drop_function_keeps_its_signature_and_a_reserved_charset_is_a_keyword() {
    assert_eq!(
        formats_as("postgres", "DROP FUNCTION sqrt (integer);\n"),
        "DROP FUNCTION sqrt(integer);\n"
    );
    assert_eq!(
        formats_as("mysql", "CREATE DATABASE d CHARACTER SET binary;\n"),
        "CREATE DATABASE d CHARACTER SET BINARY;\n"
    );
}

#[test]
fn a_table_variable_keeps_its_case() {
    assert_eq!(
        formats_as(
            "tsql",
            "DECLARE @tmpCacheTbl TABLE (a int)\nINSERT INTO @tmpCacheTbl SELECT 1\nSELECT a FROM @tmpCacheTbl\n"
        ),
        "DECLARE @tmpCacheTbl TABLE (a int)\nINSERT INTO @tmpCacheTbl SELECT 1\nSELECT a FROM @tmpCacheTbl\n"
    );
}

#[test]
fn mysql_binary_after_a_type_and_array_follow_the_case_of_names() {
    assert_eq!(
        formats_as(
            "mysql",
            "CREATE TABLE t (a varchar(7) BINARY NULL, b bigint);\n"
        ),
        "CREATE TABLE t (a varchar(7) binary NULL, b bigint);\n"
    );
    assert_eq!(
        formats_as(
            "mysql",
            "CREATE INDEX i ON t ((CAST(a AS CHAR(255) ARRAY)), b);\nselect ab from t;\n"
        ),
        "CREATE INDEX i ON t ((CAST(a AS CHAR(255) array)), b);\nSELECT ab FROM t;\n"
    );
}

#[test]
fn mysql_on_update_of_a_column_does_not_open_an_expression() {
    assert_eq!(
        formats_as(
            "mysql",
            "CREATE TABLE t (\n    a timestamp ON UPDATE CURRENT_TIMESTAMP,\n    UNIQUE KEY `c` (`name`)\n);\n"
        ),
        "CREATE TABLE t (\n    a TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,\n    UNIQUE KEY `c` (`name`)\n);\n"
    );
}

#[test]
fn tsql_xml_methods_keep_their_case_and_cast_is_a_keyword() {
    assert_eq!(
        formats_as(
            "tsql",
            "SELECT cast(N'' AS xml).value(1), LEFT(a, 1), count(1) FROM t\n"
        ),
        "SELECT\n    CAST(N'' AS xml).value(1),\n    LEFT(a, 1),\n    count(1)\nFROM t\n"
    );
}

#[test]
fn a_setting_value_true_or_false_is_a_literal_of_the_literal_case() {
    assert_eq!(
        formats_as("postgres", "SET a = false;\nSELECT NULL, TRUE;\n"),
        "SET a = false;\nSELECT\n    null,\n    true;\n"
    );
}

#[test]
fn a_referential_action_after_on_update_is_a_keyword_and_a_placeholder_is_kept() {
    assert_eq!(
        formats_as(
            "ansi",
            "CREATE TABLE t (a int, FOREIGN KEY (a) REFERENCES u (b) ON UPDATE cascade ON DELETE no ACTION);\n"
        ),
        "CREATE TABLE t (\n    a int,\n    FOREIGN KEY (a) REFERENCES u (b) ON UPDATE CASCADE ON DELETE NO ACTION\n);\n"
    );
    assert_eq!(
        formats_as(
            "postgres",
            "ALTER TABLE ${table} ALTER COLUMN c TYPE VARCHAR(256);\n"
        ),
        "ALTER TABLE ${table} ALTER COLUMN c TYPE VARCHAR(256);\n"
    );
}

#[test]
fn an_unnamed_multi_word_parameter_type_is_a_type() {
    assert_eq!(
        formats_as(
            "postgres",
            "CREATE FUNCTION f(double precision[], integer) RETURNS int AS $$ SELECT 1 $$ LANGUAGE sql;\n"
        ),
        "CREATE FUNCTION f(\n    double precision[], integer\n) RETURNS int AS $$ SELECT 1 $$ LANGUAGE sql;\n"
    );
}

#[test]
fn tsql_comments_that_open_the_file_leave_one_blank_line_like_a_batch_start() {
    let sql = "/* c */\n\n\nSELECT 1;\n";
    assert_eq!(formats_as("tsql", sql), "/* c */\n\nSELECT 1;\n");
    assert_eq!(formats_as("ansi", sql), sql);
}

#[test]
fn a_multi_line_token_does_not_make_the_line_it_ends_on_long() {
    let sql = "CREATE FUNCTION convert_ts(s VARCHAR) RETURNS TIMESTAMP AS $$\nBEGIN\n   RETURN select_something_long_here(s);\nEND\n$$ LANGUAGE plpgsql;\n";
    assert_eq!(formats_as("postgres", sql), sql);
}

#[test]
fn blank_lines_inside_a_block_comment_count_like_those_between_statements() {
    assert_eq!(
        formats_as(
            "tsql",
            "SELECT 1\n/* a\n\n\nb */\nFROM t;\n\n\n\n/* c\n\n\n\nd */\nSELECT 2;\n"
        ),
        "SELECT 1\n/* a\n\nb */\nFROM t;\n\n\n/* c\n\n\nd */\nSELECT 2;\n"
    );
}
