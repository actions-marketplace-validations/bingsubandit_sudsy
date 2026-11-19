//! The Rust rules of Java (Checkstyle): the checks that read several nodes. The checks on one
//! node's own text are pattern files in `rules/`.

use core_rules::Rule;

pub static RULES: &[&dyn Rule] = &[
    &crate::scope::UNUSED_IMPORT,
    &crate::scope::UNUSED_LOCAL,
    &crate::scope::PARAMETER_ASSIGNMENT,
];
