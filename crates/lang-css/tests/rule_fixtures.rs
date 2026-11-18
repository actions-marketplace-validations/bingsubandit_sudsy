//! `tests/rules/css/<rule>/*`: see docs/ai/development/testing.md.

#[test]
fn rule_fixtures() {
    provider_api::fixtures::rule_fixtures(&lang_css::provider(lang_css::Dialect::Css));
}
