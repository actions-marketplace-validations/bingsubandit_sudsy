//! What PHP says to the shared scope rules (`core_rules::scope_rules`): `../scope.rows` and
//! `../globals/php.txt`.
//!
//! concept: scope-tables

use core_rules::{ScopeLang, ScopeRule};

static LANG: ScopeLang =
    ScopeLang::new(include_str!("../scope.rows")).globals(include_str!("../globals/php.txt"));

pub static UNDEFINED_VARIABLE: ScopeRule = ScopeRule::undefined_variable(&LANG);
