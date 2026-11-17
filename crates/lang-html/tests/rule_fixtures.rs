//! `tests/rules/{html,vue,svelte,angular}/<rule>/*`: see docs/development/testing.md.

#[test]
fn rule_fixtures() {
    // The script of a Vue file is JavaScript, which the host's rules read.
    let js: &'static _ = Box::leak(Box::new(lang_js::provider()));
    provider_api::fixtures::rule_fixtures_dialects_with(
        &lang_html::provider(),
        &[
            &lang_html::vue_provider(),
            &lang_html::svelte_provider(),
            &lang_html::angular_provider(),
        ],
        &[js],
    );
}
