//! The builtins that are classes (`int`, `ValueError`, `type`; not the exception groups, which ruff
//! leaves out): `../globals/builtin-classes.txt`. The names Python binds before the first line of
//! a file are `../globals/python.txt`.
//!
//! concept: scope-tables

use std::collections::HashSet;
use std::sync::LazyLock;

static CLASSES: LazyLock<HashSet<&str>> = LazyLock::new(|| {
    include_str!("../globals/builtin-classes.txt")
        .lines()
        .collect()
});

pub fn is_builtin_class(name: &str) -> bool {
    CLASSES.contains(name)
}
