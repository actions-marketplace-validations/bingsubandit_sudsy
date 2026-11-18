//! The rules of Kotlin; `cargo xtask new-rule kotlin <id> --rust` adds a module under `rules/` and
//! lists it here.
//!
//! concept: none (glue: rule registration)

use core_rules::Rule;

pub static RULES: &[&dyn Rule] = &[&crate::scope::UNUSED_IMPORT];
