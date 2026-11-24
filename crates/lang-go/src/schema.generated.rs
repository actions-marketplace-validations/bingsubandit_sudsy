// Generated from lang.yaml by `cargo xtask codegen`.

#[rustfmt::skip]
pub fn option_schema() -> core_config::OptionSchema {
    core_config::OptionSchema::lazy(|| {
        core_config::OptionSchema::new(vec![
        shared::indent_style(),
        core_config::shared::final_newline(true),
        shared::end_of_line(),
        ])
        .with_presets(vec![
        core_config::Preset::new("go/revive", include_str!("../presets/revive.yaml")),
        ])
    })
}
