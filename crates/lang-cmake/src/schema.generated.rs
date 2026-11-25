// Generated from lang.yaml by `cargo xtask codegen`.
#[allow(dead_code)]
pub const LEADING_ARGUMENT_COMMANDS: core_config::OptId = core_config::opt::LEADING_ARGUMENT_COMMANDS;

#[rustfmt::skip]
pub fn option_schema() -> core_config::OptionSchema {
    core_config::OptionSchema::lazy(|| {
        core_config::OptionSchema::new(vec![
        core_config::shared::indent_size(4),
        core_config::shared::line_width(80),
        shared::indent_style(),
        core_config::shared::final_newline(true),
        shared::end_of_line(),
        core_config::shared::max_blank_lines(1, "Largest number of blank lines kept between commands and comments."),
        core_config::OptionDef::words(core_config::opt::LEADING_ARGUMENT_COMMANDS, &["set"], "Commands whose first argument stays on the line of the command when the call breaks (`set(NAME` then the values)."),
        ])
        .with_presets(vec![
        core_config::Preset::new("cmake/cmake-lint", include_str!("../presets/cmake-lint.yaml")),
        ])
    })
}
