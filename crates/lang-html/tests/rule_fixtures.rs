//! `tests/rules/{html,vue,svelte,angular}/<rule>/*`: see docs/development/testing.md.

#[test]
fn rule_fixtures() {
    provider_api::fixtures::rule_fixtures_dialects(
        &lang_html::provider(),
        &[
            &lang_html::vue_provider(),
            &lang_html::svelte_provider(),
            &lang_html::angular_provider(),
        ],
    );
}
