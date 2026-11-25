//! `no-duplicate-sections`, written against the `#sections` (a file's container of sections) and
//! `#section` (a section's header node, whose `name` slot holds the name token) grammar tags.
//!
//! concept: no-duplicate-sections
//! what: Two sections of one file with the same name.
//! rules: no-duplicate-sections
//! reads: rule:no-duplicate-sections

use crate::rule;
use crate::rule::RuleContext;
use crate::unique::Repeats;
use core_syntax::SyntaxNode;

rule! {
    pub NoDuplicateSections {
        id: "no-duplicate-sections",
        category: Correctness,
        default: Warn,
        description: "Reports a section that has the same name as an earlier section of the file (INI \
              and EditorConfig: the section header, for EditorConfig the glob).",
        why: "Readers of the format handle repeated sections in different ways: some merge them, \
              some keep only the last, so the settings of one of them may be lost or applied in \
              an order that is hard to see.\n\
              To fix, merge the two sections into one.",
        example: "[core]\nbare = false\n[core]   # reported: the same section again",
        tags: &["sections"],
    }

    fn check(&self, node: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        let grammar = ctx.file.grammar;
        let mut seen = Repeats::default();
        let headers = node
            .children()
            .filter_map(|section| section.children().next())
            .filter(|h| grammar.has_tag(h.kind(), "section"));
        for header in headers {
            let Some(name) = grammar.slot(&header, "name").and_then(|n| n.into_token()) else {
                continue;
            };
            let text = name.text();
            let message = || format!("section `[{text}]` is already defined");
            seen.check(ctx, text.to_string(), name.text_range(), message, "first defined here");
        }
    }
}
