//! The rules of PowerShell; `cargo xtask new-rule powershell <id> --rust` adds a module under
//! `rules/` and lists it here.
//!
//! concept: none (glue: rule registration)

use core_rules::Rule;

pub static RULES: &[&dyn Rule] = &[];
