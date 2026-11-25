//! `no-duplicate-case`, on the `#switch` and `#case` roles: two arms of one switch with the same
//! test (the `test` field of a case; an arm with several labels has one per label).
//!
//! concept: no-duplicate-case
//! what: Two arms of one switch with the same test.
//! rules: no-duplicate-case
//! reads: rule:no-duplicate-case

use crate::rule;
use crate::rule::RuleContext;
use crate::tree::significant_range;
use crate::unique::Repeats;
use core_syntax::{Grammar, SyntaxElement, SyntaxNode};

/// The `#case` nodes of a switch: its children, or the children of the wrapper that holds them
/// (PHP's `SwitchBraceDelimitedBody`), never those of a nested switch or arm.
fn cases(node: &SyntaxNode, g: &Grammar, out: &mut Vec<SyntaxNode>, depth: u8) {
    for child in node.children() {
        if g.has_tag(child.kind(), "case") {
            out.push(child);
        } else if depth > 0 && !g.has_tag(child.kind(), "switch") {
            cases(&child, g, out, depth - 1);
        }
    }
}

/// The tokens of an element, joined by a space.
fn words(e: &SyntaxElement, g: &Grammar) -> String {
    let tokens = match e {
        SyntaxElement::Token(t) => vec![t.clone()],
        SyntaxElement::Node(n) => n
            .descendants_with_tokens()
            .filter_map(|e| e.into_token())
            .collect(),
    };
    let words: Vec<_> = tokens
        .iter()
        .filter(|t| !g.is_trivia(t.kind()))
        .map(|t| t.text())
        .collect();
    words.join(" ")
}

rule! {
    pub NoDuplicateCase {
        id: "no-duplicate-case",
        category: Suspicious,
        default: Warn,
        description: "Reports an arm of a `switch`, `match`, `when` or `case` whose test is the same as \
              the test of an earlier arm of the same statement (`case 1:` twice). The second arm \
              is reported, the first is shown as related.",
        why: "Only the first arm with that test can run, so the second is dead code. It is \
              usually a copy that should have had another value, and the case it was meant for \
              is then unhandled.\n\
              To fix, change the test to the value it was meant for, or merge the two arms.",
        example: "switch (a) { case 1: break; case 1: break; }   // the second case is reported",
        tags: &["switch"],
    }

    fn check(&self, node: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        let g = ctx.file.grammar;
        let mut found = Vec::new();
        cases(node, g, &mut found, 2);
        let mut seen = Repeats::default();
        for case in found {
            for test in g.slots(&case, "test") {
                let range = significant_range(&case, g);
                let key = words(&test, g);
                seen.check(ctx, key, range, || "duplicate case label".to_string(), "first case");
            }
        }
    }
}
