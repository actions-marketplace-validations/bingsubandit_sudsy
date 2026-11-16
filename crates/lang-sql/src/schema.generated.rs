// Generated from lang.yaml by `cargo xtask codegen`.
#[allow(dead_code)]
pub const DIALECT: &str = "dialect";
#[allow(dead_code)]
pub const COMMA_POSITION: &str = "comma_position";
#[allow(dead_code)]
pub const KEYWORD_CASE: &str = "keyword_case";
#[allow(dead_code)]
pub const IDENTIFIER_CASE: &str = "identifier_case";
#[allow(dead_code)]
pub const FUNCTION_CASE: &str = "function_case";
#[allow(dead_code)]
pub const LITERAL_CASE: &str = "literal_case";
#[allow(dead_code)]
pub const TYPE_CASE: &str = "type_case";

#[rustfmt::skip]
pub fn option_schema() -> core_config::OptionSchema {
    core_config::OptionSchema::new(vec![
        core_config::OptionDef::choice("dialect", &["ansi", "postgres", "mysql", "tsql"], "ansi", "The SQL dialect the file is parsed as."),
        core_config::shared::line_width(80),
        shared::indent_style(),
        core_config::shared::indent_size(4),
        core_config::shared::final_newline(true),
        shared::end_of_line(),
        core_config::OptionDef::choice("comma_position", &["trailing", "leading"], "trailing", "Where a comma goes when a list is broken over lines: at the end of a line, or at the start of the next one."),
        core_config::OptionDef::choice("keyword_case", &["consistent", "upper", "lower", "capitalise"], "consistent", "Case of keywords: `consistent` follows the first one written (all that follow are brought to its case once one differs), the others set it."),
        core_config::OptionDef::choice("identifier_case", &["consistent", "upper", "lower", "capitalise"], "consistent", "Case of unquoted identifiers: `consistent` follows the first one written (all that follow are brought to its case once one differs), the others set it."),
        core_config::OptionDef::choice("function_case", &["consistent", "upper", "lower", "capitalise"], "consistent", "Case of function names: `consistent` follows the first one written (all that follow are brought to its case once one differs), the others set it."),
        core_config::OptionDef::choice("literal_case", &["consistent", "upper", "lower", "capitalise"], "consistent", "Case of `NULL`, `TRUE` and `FALSE`: `consistent` follows the first one written (all that follow are brought to its case once one differs), the others set it."),
        core_config::OptionDef::choice("type_case", &["consistent", "upper", "lower", "capitalise"], "consistent", "Case of data types: `consistent` follows the first one written (all that follow are brought to its case once one differs), the others set it."),
    ])
    .with_presets(vec![
    ])
}
