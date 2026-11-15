// Generated from lang.yaml by `cargo xtask codegen`.
#[allow(dead_code)]
pub const DIALECT: &str = "dialect";

#[rustfmt::skip]
pub fn option_schema() -> core_config::OptionSchema {
    core_config::OptionSchema::new(vec![
        core_config::OptionDef::choice("dialect", &["ansi", "postgres", "mysql", "tsql"], "ansi", "The SQL dialect the file is parsed as."),
    ])
    .with_presets(vec![
    ])
}
