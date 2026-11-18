//! What Java says to the shared scope rules (`core_rules::scope_rules`): `../scope.rows`.
//!
//! concept: scope-tables

use core_rules::{ScopeLang, ScopeRule};

static LANG: ScopeLang = ScopeLang::new(include_str!("../scope.rows"));

pub static UNUSED_IMPORT: ScopeRule = ScopeRule::unused_import(&LANG);
pub static UNUSED_LOCAL: ScopeRule = ScopeRule::unused_local(&LANG);
pub static PARAMETER_ASSIGNMENT: ScopeRule = ScopeRule::parameter_assignment(&LANG);
