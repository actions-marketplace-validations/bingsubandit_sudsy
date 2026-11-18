//! The Rust rules of Java (Checkstyle): the checks that read several nodes. The checks on one
//! node's own text are pattern files in `rules/`.
//!
//! concept: new modifier-order
//! what: The modifiers of a declaration are not in the order of the Java Language Specification: `public protected private abstract default static sealed non-sealed final transien

use crate::generated::kind as k;
use core_rules::{Rule, RuleContext, rule};
use core_syntax::{SyntaxNode, SyntaxNodeExt};

pub static RULES: &[&dyn Rule] = &[
    &ModifierOrder,
    &crate::scope::UNUSED_IMPORT,
    &crate::scope::UNUSED_LOCAL,
    &crate::scope::PARAMETER_ASSIGNMENT,
];

const MODIFIER_ORDER: [&str; 14] = [
    "public",
    "protected",
    "private",
    "abstract",
    "default",
    "static",
    "sealed",
    "non-sealed",
    "final",
    "transient",
    "volatile",
    "synchronized",
    "native",
    "strictfp",
];

rule! {
    pub ModifierOrder {
        id: "modifier-order",
        category: Style,
        default: Off,
        description: "The modifiers of a declaration are not in the order of the Java Language \
                  Specification: `public protected private abstract default static sealed non-sealed \
                  final transient volatile synchronized native strictfp` (Checkstyle ModifierOrder). \
                  No fix: the check does not rewrite code.",
        example: "static public final int X = 1;   // public static final",
        kinds: &[k::MODIFIERS],
    }

    fn check(&self, node: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        let mut top = 0;
        for t in node.tokens().filter(|t| t.kind() == k::KEYWORD) {
            let Some(rank) = MODIFIER_ORDER.iter().position(|m| *m == t.text()) else { continue };
            if rank < top {
                ctx.report(t.text_range(), format!("`{}` is out of order", t.text()));
                return;
            }
            top = rank;
        }
    }
}
