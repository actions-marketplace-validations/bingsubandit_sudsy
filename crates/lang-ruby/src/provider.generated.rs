// Generated from lang.yaml by `cargo xtask codegen`.
use core_syntax::grammars;
use provider_api::{ParserProvider, ParserSpec};

core_syntax::include_generated!();

static FORMAT: core_format::rules::LazyRules = core_format::rules::LazyRules::new(|| grammars::RUBY.get(), include_str!("../format.rules")).source_indent().facts(core_format::rules::RuleFacts { exact_end: &["Data"], keep_comment_gap: true, ..core_format::rules::RuleFacts::DEFAULT });

#[rustfmt::skip]
pub fn provider() -> ParserProvider {
    ParserProvider::new(ParserSpec { crate_name: env!("CARGO_PKG_NAME"), grammar: grammars::RUBY, extensions: &["rb", "rake", "gemspec", "ru"], file_names: &["Gemfile", "Rakefile", "Guardfile", "Capfile"], schema: option_schema(), formatter: &FORMAT, hooks: &core_rules::DEFAULT_HOOKS, rules: rules::RULES, pattern_files: provider_api::pattern_files!(), families: rules::FAMILIES }, adapter::parse_file)
}
