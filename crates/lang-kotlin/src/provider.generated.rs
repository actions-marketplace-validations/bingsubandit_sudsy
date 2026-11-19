// Generated from lang.yaml by `cargo xtask codegen`.
use core_syntax::grammars;
use provider_api::{ParserProvider, ParserSpec};

core_syntax::include_generated!();

#[rustfmt::skip]
pub fn provider() -> ParserProvider {
    ParserProvider::new(ParserSpec { crate_name: env!("CARGO_PKG_NAME"), grammar: grammars::KOTLIN, extensions: &["kt", "kts"], file_names: &[], schema: option_schema(), formatter: &KotlinFormatter, hooks: &core_rules::DEFAULT_HOOKS, rules: rules::RULES, pattern_files: provider_api::pattern_files!(), families: &[] }, adapter::parse_file)
}

pub const LIMITS: core_syntax::scan::Limits = core_syntax::scan::Limits::new(256, 16, 2000, "!-+@", Some((b'@', 12)));
