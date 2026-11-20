// Generated from lang.yaml by `cargo xtask codegen`.
#[allow(dead_code)]
pub const MEMBER_CHAIN_MIN_CALLS: core_config::OptId = core_config::opt::MEMBER_CHAIN_MIN_CALLS;
#[allow(dead_code)]
pub const MEMBER_CHAIN_KEEP_ACCESSES: core_config::OptId = core_config::opt::MEMBER_CHAIN_KEEP_ACCESSES;
#[allow(dead_code)]
pub const MEMBER_CHAIN_BREAK_ON_BLOCK: core_config::OptId = core_config::opt::MEMBER_CHAIN_BREAK_ON_BLOCK;
#[allow(dead_code)]
pub const MEMBER_CHAIN_HEAD_CALLS: core_config::OptId = core_config::opt::MEMBER_CHAIN_HEAD_CALLS;
#[allow(dead_code)]
pub const TYPE_USE_ANNOTATIONS: core_config::OptId = core_config::opt::TYPE_USE_ANNOTATIONS;
#[allow(dead_code)]
pub const MEMBER_CHAIN_SHORT_HEAD: core_config::OptId = core_config::opt::MEMBER_CHAIN_SHORT_HEAD;
#[allow(dead_code)]
pub const MEMBER_CHAIN_INDENT: core_config::OptId = core_config::opt::MEMBER_CHAIN_INDENT;
#[allow(dead_code)]
pub const BINARY_OPERATOR_LINE: core_config::OptId = core_config::opt::BINARY_OPERATOR_LINE;
#[allow(dead_code)]
pub const BINARY_INDENT: core_config::OptId = core_config::opt::BINARY_INDENT;
#[allow(dead_code)]
pub const BINARY_HUG: core_config::OptId = core_config::opt::BINARY_HUG;
#[allow(dead_code)]
pub const HUG_FIRST_ARGUMENT: core_config::OptId = core_config::opt::HUG_FIRST_ARGUMENT;
#[allow(dead_code)]
pub const HUG_LAST_ARGUMENT: core_config::OptId = core_config::opt::HUG_LAST_ARGUMENT;
#[allow(dead_code)]
pub const INDENT_NESTED_BINARY: core_config::OptId = core_config::opt::INDENT_NESTED_BINARY;
#[allow(dead_code)]
pub const ASSIGNMENT_LAYOUT: core_config::OptId = core_config::opt::ASSIGNMENT_LAYOUT;
#[allow(dead_code)]
pub const ASSIGNMENT_CONDITIONAL: core_config::OptId = core_config::opt::ASSIGNMENT_CONDITIONAL;
#[allow(dead_code)]
pub const ASSIGNMENT_INDENT: core_config::OptId = core_config::opt::ASSIGNMENT_INDENT;
#[allow(dead_code)]
pub const CONDITIONAL_INDENT: core_config::OptId = core_config::opt::CONDITIONAL_INDENT;
#[allow(dead_code)]
pub const CONDITIONAL_LAYOUT: core_config::OptId = core_config::opt::CONDITIONAL_LAYOUT;

#[rustfmt::skip]
pub fn option_schema() -> core_config::OptionSchema {
    core_config::OptionSchema::lazy(|| {
        core_config::OptionSchema::new(vec![
        core_config::shared::indent_size(2),
        core_config::shared::line_width(100),
        shared::indent_style(),
        core_config::shared::final_newline(true),
        shared::end_of_line(),
        core_config::shared::member_chain_min_calls(2),
        core_config::shared::member_chain_keep_accesses("head"),
        core_config::shared::member_chain_break_on_block("any"),
        core_config::shared::member_chain_head_calls(&["stream"]),
        core_config::shared::type_use_annotations(&["org.jspecify.annotations.Nullable", "org.jspecify.annotations.NonNull", "org.checkerframework.*.Nullable", "org.checkerframework.*.NonNull"]),
        core_config::shared::member_chain_short_head(4),
        core_config::shared::member_chain_indent(2),
        core_config::shared::binary_operator_line("start"),
        core_config::shared::binary_indent(2),
        core_config::shared::binary_hug("never"),
        core_config::shared::hug_first_argument(false),
        core_config::shared::hug_last_argument("sole"),
        core_config::shared::indent_nested_binary(true),
        core_config::shared::assignment_layout("break"),
        core_config::shared::assignment_conditional("always"),
        core_config::shared::assignment_indent(2),
        core_config::shared::conditional_indent(2),
        core_config::shared::conditional_layout("indented"),
        ])
        .with_presets(vec![
        core_config::Preset::new("java/google", include_str!("../presets/google.yaml")),
        ])
    })
}
