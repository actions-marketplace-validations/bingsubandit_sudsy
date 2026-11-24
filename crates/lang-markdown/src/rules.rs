//! The Rust rules of Markdown (the heading checks that need the whole document); the rules that
//! fit a pattern are files in `rules/`.
//!
//! concept: new heading-increment, new single-title
//! what: heading-increment: A heading is more than one level deeper than the heading before it (an `h4` right after an `h2`)
//! what: single-title: A document has more than one top-level heading (markdownlint MD025)

use crate::format::{heading_level, heading_range};
use crate::generated::kind;
use core_rules::heading_outline::{self, Outline, Several};
use core_rules::rule::FileContext;
use core_rules::{Rule, RuleContext, rule};
use core_syntax::{SyntaxNode, TextRange};

pub static RULES: &[&dyn Rule] = &[&HeadingIncrement, &SingleTitle];

/// The outline checks of Markdown, each rule turns one on.
const OUTLINE: Outline = Outline {
    first_is_top: false,
    skips: false,
    several: Several::Off,
    label: |n| format!("h{n}"),
    many: "the document has more than one top-level heading",
};

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
        heading_outline::check(headings(file).into_iter(), &Outline { skips: true, ..OUTLINE }, ctx);
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
        heading_outline::check(headings(file).into_iter(), &Outline { several: Several::Later, ..OUTLINE }, ctx);
    }
}
