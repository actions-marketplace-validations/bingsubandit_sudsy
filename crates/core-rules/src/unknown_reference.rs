//! A reference to a name that nothing declares: a `needs` entry naming no job, a `COPY --from` naming
//! no stage. The language lists its declarations and references in source order with the range each
//! report goes to; the helper compares them.
//!
//! concept: unknown-reference
//! what: A name used where a declared name is required, and no declaration of it (before the use, when order counts).
//! tools: actionlint job-needs

use crate::rule::RuleContext;
use core_syntax::TextRange;

/// One entry of a language's list, in source order.
pub enum Item {
    /// A name this file declares.
    Declare(String),
    /// A use of a name, and a `tag` the language reads back to place and word the report.
    Use { name: String, tag: usize },
}

/// What counts as declared.
pub struct Scope {
    /// Only declarations before the use count.
    pub earlier_only: bool,
    /// Names are equal ignoring ASCII case.
    pub ignore_case: bool,
}

/// Reports every use of a name that no declaration in scope has, placed and worded by
/// `report(tag, name)`.
/// `exempt(name, declared)` names the uses that need no declaration (`declared`: how many
/// declarations came before).
pub fn check(
    items: &[Item],
    scope: &Scope,
    exempt: impl Fn(&str, usize) -> bool,
    report: impl Fn(usize, &str) -> (TextRange, String),
    ctx: &mut RuleContext<'_>,
) {
    let same = |a: &str, b: &str| {
        if scope.ignore_case {
            a.eq_ignore_ascii_case(b)
        } else {
            a == b
        }
    };
    let mut known: Vec<&str> = Vec::new();
    if !scope.earlier_only {
        known.extend(items.iter().filter_map(|i| match i {
            Item::Declare(n) => Some(n.as_str()),
            Item::Use { .. } => None,
        }));
    }
    let mut count = 0;
    for item in items {
        match item {
            Item::Declare(name) => {
                count += 1;
                if scope.earlier_only {
                    known.push(name);
                }
            }
            Item::Use { name, tag } => {
                if !known.iter().any(|d| same(d, name)) && !exempt(name, count) {
                    let (at, message) = report(*tag, name);
                    ctx.report(at, message);
                }
            }
        }
    }
}
