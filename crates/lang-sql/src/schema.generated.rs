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
        core_config::OptionDef::choice("comma_position", &["trailing", "leading"], "trailing", "Where the comma of a list that breaks goes: at the end of a line or at the start of the next."),
        core_config::OptionDef::choice("keyword_case", &["consistent", "upper", "lower", "capitalise", "preserve"], "consistent", "Capitalisation of keywords: one spelling throughout the file (the first one's), or the named case."),
        core_config::OptionDef::choice("identifier_case", &["consistent", "upper", "lower", "capitalise", "preserve"], "consistent", "Capitalisation of unquoted identifiers."),
        core_config::OptionDef::choice("function_case", &["consistent", "upper", "lower", "capitalise", "preserve"], "consistent", "Capitalisation of function names."),
        core_config::OptionDef::choice("literal_case", &["consistent", "upper", "lower", "capitalise", "preserve"], "consistent", "Capitalisation of `NULL`, `TRUE` and `FALSE`."),
        core_config::OptionDef::choice("type_case", &["consistent", "upper", "lower", "capitalise", "preserve"], "consistent", "Capitalisation of data types."),
    ])
    .with_presets(vec![
    ])
}
