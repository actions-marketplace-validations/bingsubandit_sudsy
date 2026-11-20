//! `modifier-order`: the keyword modifiers of a declaration (the `#modifiers` nodes) stand in the
//! order the language's style wants. The order is the `order` row of the `[modifier-order]`
//! section of the language's `format.rules`; the option `report: list` puts the finding on the
//! first token of the modifier list, not on the keyword that is out of place; a keyword not in it, and every annotation, keeps its
//! place. The fix swaps the keywords' texts, so comments and blanks stay where they are, and the
//! formatter runs it before layout (`Rule::formats`).
//!
//! concept: modifier-order
//! what: The modifier keywords of a declaration in the order the style wants.
//! tools: Checkstyle ModifierOrder, google-java-format, ktlint modifier-order
//! rules: modifier-order
//! reads: rules:modifier-order.order, tag:modifiers

use crate::diagnostic::Edit;
use crate::options::{Check, Opt, OptionDecl};
use crate::rule;
use crate::rule::{FileContext, RuleContext};

rule! {
    pub ModifierOrder {
        id: "modifier-order",
        category: Style,
        default: Off,
        fix: Safe,
        description: "The modifiers of a declaration are not in the order the style wants \
                  (Java: `public protected private abstract default static sealed non-sealed final \
                  transient volatile synchronized native strictfp`; Kotlin: ktlint's order). The \
                  fix puts them in order.",
        example: "static public final int X = 1;   // public static final",
        options: [OptionDecl {
            name: "report",
            default: Opt::Str("keyword"),
            check: Check::OneOf(&["keyword", "list"]),
            description: "Where a finding is reported: on the keyword that is out of place, or on \
                      the first token of the modifier list.",
        }],
        format: true,
    }

    fn format_kinds(&self, grammar: &core_syntax::Grammar) -> Vec<core_syntax::SyntaxKind> {
        grammar.nodes_with_tag("modifiers").to_vec()
    }

    fn check_file(&self, file: &FileContext<'_>, ctx: &mut RuleContext<'_>) {
        let order = ctx.parsed("modifier-order", || {
            core_format::rules::settings(ctx.rows, "modifier-order")
                .into_iter()
                .filter(|(k, _)| *k == "order")
                .flat_map(|(_, v)| v.split_whitespace().map(str::to_string).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        });
        if order.is_empty() {
            return;
        }
        let at_list = ctx.options.string("report") == "list";
        let rank = |text: &str| order.iter().position(|m| m == text);
        for node in ctx.nodes(file.grammar.nodes_with_tag("modifiers")) {
            // (most declarations are in order: no allocation until one is not)
            let mut last = 0;
            let mut out_of_order = false;
            for t in node.children_with_tokens().filter_map(|e| e.into_token()) {
                if let Some(r) = rank(t.text()) {
                    out_of_order |= r < last;
                    last = r;
                }
            }
            if !out_of_order {
                continue;
            }
            let slots: Vec<_> = node
                .children_with_tokens()
                .filter_map(|e| e.into_token())
                .filter_map(|t| rank(t.text()).map(|r| (r, t)))
                .collect();
            let Some(first) = slots.windows(2).find(|w| w[1].0 < w[0].0).map(|w| &w[1].1) else {
                continue;
            };
            let mut sorted: Vec<_> = slots.iter().collect();
            sorted.sort_by_key(|(r, _)| *r);
            let edits = slots
                .iter()
                .zip(&sorted)
                .filter(|((_, a), (_, b))| a.text() != b.text())
                .map(|((_, a), (_, b))| Edit::replace(a.text_range(), b.text()))
                .collect();
            let at = if at_list {
                let start = crate::tree::code_start(&node, file.grammar);
                core_syntax::TextRange::at(start, 1.into())
            } else {
                first.text_range()
            };
            ctx.report_fix(
                at,
                format!("the modifier {} is out of order", first.text()),
                ("Put the modifiers in order", edits),
            );
        }
    }
}
