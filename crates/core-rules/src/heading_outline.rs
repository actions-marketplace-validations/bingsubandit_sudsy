//! The outline of a document's headings: the first heading is a top-level one, a level goes down
//! one at a time, and a document has one top-level heading. The language passes its headings (level
//! and range) and what it checks; the helper decides which heading is reported and says why.
//!
//! concept: heading-outline
//! what: Headings that break the outline: a first heading below the top level, a skipped level, more than one top-level heading.
//! tools: markdownlint MD001 MD025, html-validate heading-level

use crate::rule::RuleContext;
use core_syntax::TextRange;

/// Which top-level headings are reported when a document has several.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Several {
    /// None: the check is off.
    Off,
    /// The second and later ones.
    Later,
    /// Every one, the first included.
    All,
}

/// The checks of one rule and the way a language names a heading.
pub struct Outline {
    pub first_is_top: bool,
    pub skips: bool,
    pub several: Several,
    /// The name a message uses for a level (`<h3>`, `h3`).
    pub label: fn(usize) -> String,
    /// The message about several top-level headings.
    pub many: &'static str,
}

/// Reports the headings `(level, range)` of a document, in order, that break the outline.
pub fn check(
    headings: impl Iterator<Item = (usize, TextRange)>,
    outline: &Outline,
    ctx: &mut RuleContext<'_>,
) {
    let label = outline.label;
    let mut previous = 0;
    let (mut first_top, mut tops) = (None, 0);
    for (level, range) in headings {
        if previous == 0 && level != 1 && outline.first_is_top {
            ctx.report(
                range,
                format!(
                    "the first heading is `{}`, not `{}`",
                    label(level),
                    label(1)
                ),
            );
        } else if previous != 0 && level > previous + 1 && outline.skips {
            ctx.report(
                range,
                format!(
                    "`{}` skips from `{}`: a level goes down one at a time",
                    label(level),
                    label(previous)
                ),
            );
        }
        if level == 1 && outline.several != Several::Off {
            tops += 1;
            // The first top-level heading is reported once a second one shows the document has several.
            match (tops, first_top) {
                (1, _) => first_top = Some(range),
                (2, Some(at)) if outline.several == Several::All => {
                    ctx.report(at, outline.many);
                    ctx.report(range, outline.many);
                }
                _ => {
                    ctx.report(range, outline.many);
                }
            }
        }
        previous = level;
    }
}
