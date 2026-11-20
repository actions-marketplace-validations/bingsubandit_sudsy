//! What the shared rules ask of SQL: its words per dialect and the case it folds names to.
//!
//! The words are sqlfluff's keyword sets of the dialects (`names.txt`, one flag letter per
//! dialect: `a` ansi, `p` postgres, `m` mysql, `t` tsql).
//!
//! concept: none (glue: rule hooks)

use core_rules::word_table::WordTable;
use core_rules::{Fold, RuleHooks};

pub struct SqlHooks;

static NAMES: WordTable = WordTable::new(
    include_str!("names.txt"),
    &[
        ("ansi", 'a'),
        ("postgres", 'p'),
        ("mysql", 'm'),
        ("tsql", 't'),
    ],
);

impl RuleHooks for SqlHooks {
    fn is_reserved(&self, version: Option<&str>, upper: &str) -> bool {
        NAMES.reserved(version, upper)
    }

    fn is_keyword(&self, version: Option<&str>, upper: &str) -> bool {
        NAMES.keyword(version, upper)
    }

    fn fold(&self, version: Option<&str>) -> Fold {
        match version {
            Some("postgres") => Fold::Lower,
            Some("mysql") => Fold::None,
            _ => Fold::Upper,
        }
    }
}
