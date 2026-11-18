//! The Rust rules of Protobuf: none, every check is a pattern file in `rules/`.
//!
//! concept: none (glue: rule registration)

use core_rules::Rule;

pub static RULES: &[&dyn Rule] = &[];
