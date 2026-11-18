//! What Kotlin says to the shared scope rules (`core_rules::scope_rules`): `../scope.rows`.

use core_rules::{ScopeLang, ScopeRule};

static LANG: ScopeLang = ScopeLang::new(include_str!("../scope.rows"));

pub static UNUSED_IMPORT: ScopeRule = ScopeRule::unused_import(&LANG);
