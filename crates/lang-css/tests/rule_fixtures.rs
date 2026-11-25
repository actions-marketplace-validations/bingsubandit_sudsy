//! `tests/rules/css/<rule>/*` and, for SCSS, `tests/rules/scss/<rule>/*`: see
//! docs/ai/development/testing.md.

#[test]
fn rule_fixtures() {
    let scss = lang_css::provider(lang_css::Dialect::Scss);
    provider_api::fixtures::rule_fixtures_dialects(
        &lang_css::provider(lang_css::Dialect::Css),
        &[&scss],
    );
}
