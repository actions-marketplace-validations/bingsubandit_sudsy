//! Code that is not a whole file: a host (HTML, Vue, Svelte) hands one expression, which the
//! provider parses as the only element of `[ ]` (`parse`) and prints here without the brackets.
//!
//! concept: none (glue: formatter dispatch)

use super::util::{R, shape_error};
use crate::generated::kind as k;
use core_format::{FormatCtx, doc::Doc};
use core_syntax::SyntaxNode;

/// The `embed_mode` option the host's engine sets (`EmbedMode::name`).
pub(crate) const EXPRESSION: &str = "expression";
pub(crate) const NG_EXPRESSION: &str = "ng-expression";
pub(crate) const NG_NESTED: &str = "ng-nested";

/// Is the code one embedded expression (of a host or of an Angular template)?
pub(crate) fn is_expression(options: &core_config::ResolvedOptions) -> bool {
    matches!(
        options.text("embed_mode"),
        EXPRESSION | NG_EXPRESSION | NG_NESTED
    )
}

/// Is `node` the expression of an embedded expression (the only element of the `[ ]`)? It stands
/// where a root expression does: no parentheses for an assignment.
pub(crate) fn is_root(cx: &FormatCtx<'_>, node: &SyntaxNode) -> bool {
    node.parent().is_some_and(|a| {
        a.kind() == k::ARRAY_EXPRESSION
            && a.parent()
                .is_some_and(|s| s.kind() == k::EXPRESSION_STATEMENT)
            && is_expression(cx.options)
    })
}

/// [`is_root`], and the operands of a binary are not indented (Angular's nested expressions are).
pub(crate) fn is_unindented_root(cx: &FormatCtx<'_>, node: &SyntaxNode) -> bool {
    is_root(cx, node) && !cx.options.is("embed_mode", NG_NESTED)
}

pub(crate) fn format(
    node: &SyntaxNode,
    cx: &FormatCtx<'_>,
) -> Result<Option<Doc>, core_format::FormatError> {
    if node.kind() != k::PROGRAM {
        return Ok(None);
    }
    let embedded = is_expression(cx.options);
    if !embedded {
        return Ok(None);
    }
    let mut items = node
        .descendants()
        .find(|n| n.kind() == k::ARRAY_EXPRESSION)
        .map(|a| a.children().collect::<Vec<_>>())
        .unwrap_or_default();
    let (Some(expr), true) = (items.pop(), items.is_empty()) else {
        return Err(shape_error("expression"));
    };
    let doc: R = cx.format(&expr);
    doc.map(|d| Some(core_format::doc::group(d)))
}
