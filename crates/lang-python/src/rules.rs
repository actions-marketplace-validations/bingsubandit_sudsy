//! The Rust rules of Python; rules that fit a pattern are files in `rules/`.

use crate::generated::kind as k;
use core_rules::{Rule, RuleContext, rule};
use core_syntax::{SyntaxElement, SyntaxNode};

pub static RULES: &[&dyn Rule] = &[&IsLiteral];

/// A string, bytes or number literal (a negative number too).
fn is_literal(n: &SyntaxNode) -> bool {
    match n.kind() {
        k::EXPR_STRING_LITERAL | k::EXPR_BYTES_LITERAL | k::EXPR_NUMBER_LITERAL => true,
        k::EXPR_UNARY_OP => {
            n.first_token().is_some_and(|t| t.kind() == k::MINUS)
                && n.first_child()
                    .is_some_and(|c| c.kind() == k::EXPR_NUMBER_LITERAL)
        }
        _ => false,
    }
}

rule! {
    pub IsLiteral {
        id: "is-literal",
        category: Correctness,
        default: Warn,
        description: "An identity test (`is`, `is not`) with a string, bytes or number literal.",
        example: "x is \"a\"",
        kinds: &[k::EXPR_COMPARE],
    }

    fn check(&self, node: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        let parts: Vec<SyntaxElement> = node
            .children_with_tokens()
            .filter(|e| !matches!(e.kind(), k::WHITESPACE | k::NEWLINE | k::COMMENT | k::CONTINUATION))
            .collect();
        for (i, p) in parts.iter().enumerate() {
            if p.kind() != k::IS {
                continue;
            }
            let before = i.checked_sub(1).and_then(|j| parts[j].as_node());
            let after = parts[i + 1..]
                .iter()
                .find(|e| e.kind() != k::NOT)
                .and_then(|e| e.as_node());
            if before.is_some_and(is_literal) || after.is_some_and(is_literal) {
                ctx.report(node.text_range(), "use `==` to compare with a literal, not `is`");
                return;
            }
        }
    }
}
