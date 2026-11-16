//! `tests/rules/sql/<rule>/*`: see docs/development/testing.md.

#[test]
fn rule_fixtures() {
    provider_api::fixtures::rule_fixtures(&lang_sql::provider());
}
