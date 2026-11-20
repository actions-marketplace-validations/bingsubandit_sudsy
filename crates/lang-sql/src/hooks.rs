//! What the shared rules ask of SQL: its words per dialect and the case it folds names to.
//!
//! concept: none (glue: rule hooks)

use crate::parse::Dialect;
use core_rules::{Fold, RuleHooks};

pub struct SqlHooks;

fn dialect(version: Option<&str>) -> Dialect {
    Dialect::from_name(version.unwrap_or("ansi"))
}

impl RuleHooks for SqlHooks {
    fn is_reserved(&self, version: Option<&str>, upper: &str) -> bool {
        dialect(version).is_reserved(upper)
    }

    fn is_keyword(&self, version: Option<&str>, upper: &str) -> bool {
        dialect(version).is_keyword(upper)
    }

    fn fold(&self, version: Option<&str>) -> Fold {
        match version {
            Some("postgres") => Fold::Lower,
            Some("mysql") => Fold::None,
            _ => Fold::Upper,
        }
    }
}
