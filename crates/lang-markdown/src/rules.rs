//! The Rust rules of Markdown (the heading checks that need the whole document); the rules that
//! fit a pattern are files in `rules/`.
//!
//! concept: new heading-increment, new single-title
//! what: heading-increment: A heading is more than one level deeper than the heading before it (an `h4` right after an `h2`)
//! what: single-title: A document has more than one top-level heading (markdownlint MD025)

use crate::format::{heading_level, heading_range};
use crate::generated::kind;
use core_rules::rule::FileContext;
use core_rules::{Rule, RuleContext, rule};
use core_syntax::{SyntaxNode, TextRange};

pub static RULES: &[&dyn Rule] = &[&HeadingIncrement, &SingleTitle];

/// Every heading with its level and the range of its text.
fn headings(file: &FileContext<'_>) -> Vec<(usize, TextRange)> {
    file.root
        .descendants()
        .filter(|n| n.kind() == kind::HEADING)
        .map(|h: SyntaxNode| (heading_level(file.text, &h), heading_range(file.text, &h)))
        .collect()
}

rule! {
    pub HeadingIncrement {
        id: "heading-increment",
        category: Style,
        default: Off,
        description: "A heading is more than one level deeper than the heading before it (an `h4` \
                  right after an `h2`). Going back up by any number of levels is fine \
                  (markdownlint MD001).",
        example: "# One\n\n### Three   // reported",
    }

    fn check_file(&self, file: &FileContext<'_>, ctx: &mut RuleContext<'_>) {
        let mut previous = 0;
        for (level, range) in headings(file) {
            if previous > 0 && level > previous + 1 {
                ctx.report(
                    range,
                    format!("heading level should be {} (it follows level {previous})", previous + 1),
                );
            }
            previous = level;
        }
    }
}

rule! {
    pub SingleTitle {
        id: "single-title",
        category: Style,
        default: Off,
        description: "A document has more than one top-level heading (markdownlint MD025). The \
                  second and later `h1` are reported.",
        example: "# One\n\n# Two   // reported",
    }

    fn check_file(&self, file: &FileContext<'_>, ctx: &mut RuleContext<'_>) {
        for (_, range) in headings(file).into_iter().filter(|h| h.0 == 1).skip(1) {
            ctx.report(range, "the document has more than one top-level heading");
        }
    }
}
