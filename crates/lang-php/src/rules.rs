//! The rules of PHP; `cargo xtask new-rule php <id> --rust` adds a module under `rules/` and
//! lists it here.
//!
//! concept: none (glue: rule registration)

use core_rules::Rule;

pub static RULES: &[&dyn Rule] = &[&crate::scope::UNDEFINED_VARIABLE];
