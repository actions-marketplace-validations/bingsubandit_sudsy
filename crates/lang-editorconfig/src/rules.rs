//! The Rust rules of EditorConfig; syntax rules that fit a pattern are files in `rules/`.
//!
//! concept: none (glue: rule registration)

mod properties;

use core_rules::Rule;

pub static RULES: &[&dyn Rule] = &[
    &properties::UnknownProperty,
    &properties::InvalidPropertyValue,
];
