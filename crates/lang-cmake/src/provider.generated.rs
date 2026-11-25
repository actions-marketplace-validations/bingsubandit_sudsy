// Generated from lang.yaml by `cargo xtask codegen`.
use core_syntax::grammars;
use provider_api::{ParserProvider, ParserSpec};

core_syntax::include_generated!();

#[rustfmt::skip]
pub fn provider() -> ParserProvider {
    ParserProvider::new(ParserSpec { crate_name: env!("CARGO_PKG_NAME"), grammar: grammars::CMAKE, extensions: &["cmake"], file_names: &["CMakeLists.txt"], schema: option_schema(), formatter: &core_format::VerbatimFormatter { exact_end: &[] }, hooks: &core_rules::BlockHooks(&blocks::BLOCKS), rules: &[], pattern_files: provider_api::pattern_files!(), families: &[] }, parser::parse_file).without(&["*.h.cmake", "*.hpp.cmake", "*.c.cmake", "*.cpp.cmake", "*.in.cmake", "*.pc.cmake", "*.sh.cmake", "*.fish.cmake", "*.bat.cmake", "*.properties.cmake", "*.gradle.cmake", "*.xml.cmake", "*.json.cmake", "*.py.cmake"])
}
