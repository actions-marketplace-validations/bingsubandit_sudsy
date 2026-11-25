// Generated from lang.yaml by `cargo xtask codegen`.
#[allow(dead_code)]
pub const SKIP_GENERATED_LINT: core_config::OptId = core_config::opt::SKIP_GENERATED_LINT;
#[allow(dead_code)]
pub const GENERATED_HEADER_MARKERS: core_config::OptId = core_config::opt::GENERATED_HEADER_MARKERS;

#[rustfmt::skip]
pub fn option_schema() -> core_config::OptionSchema {
    core_config::OptionSchema::lazy(|| {
        core_config::OptionSchema::new(vec![
        shared::indent_style(),
        core_config::shared::final_newline(true),
        shared::end_of_line(),
        core_config::shared::skip_generated_lint(true),
        core_config::shared::generated_header_markers(&["code generated *do not edit."]),
        ])
        .with_presets(vec![
        core_config::Preset::new("go/revive", include_str!("../presets/revive.yaml")),
        ])
    })
}
