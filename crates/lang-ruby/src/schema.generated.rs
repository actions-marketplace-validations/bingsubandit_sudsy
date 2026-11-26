// Generated from lang.yaml by `cargo xtask codegen`.

#[rustfmt::skip]
pub fn option_schema() -> core_config::OptionSchema {
    core_config::OptionSchema::lazy(|| {
        core_config::OptionSchema::new(vec![
        core_config::shared::indent_size(2),
        core_config::shared::line_width(120),
        shared::indent_style(),
        core_config::shared::final_newline(true),
        core_config::shared::max_blank_lines(1, "Largest number of blank lines kept between lines (RuboCop Layout/EmptyLines)."),
        shared::end_of_line(),
        ])
        .with_presets(vec![
        core_config::Preset::new("ruby/rubocop", include_str!("../presets/rubocop.yaml")),
        ])
    })
}
