//! JSON formatting. Almost everything is the shared tag defaults; this file only maps the JSON
//! options onto the list shape for objects and arrays and handles the document's end.

use crate::generated::kind as k;
use core_config::{ResolvedOptions, shared};
use core_format::shapes::{ListLayout, ListStyle, TrailingComma, delimited_list};
use core_format::{Doc, FormatCtx, FormatError, NodeFormatter};
use core_syntax::SyntaxNode;

include!("schema.generated.rs");

pub struct JsonFormatter;

fn list_style(options: &ResolvedOptions, is_object: bool) -> ListStyle {
    let jsonc = dialect(options) == Dialect::Jsonc;
    ListStyle {
        padding: options.bool(if is_object {
            shared::BRACKET_SPACING
        } else {
            shared::ARRAY_BRACKET_SPACING
        }),
        trailing_comma: if jsonc && trailing_commas(options) == TrailingCommas::Multiline {
            TrailingComma::WhenBroken
        } else {
            TrailingComma::Never
        },
        layout: match collapse(options) {
            Collapse::Always => ListLayout::Fit,
            Collapse::Never => ListLayout::Break,
            Collapse::OpeningBreak if is_object => ListLayout::PreserveFirstBreak,
            Collapse::OpeningBreak => ListLayout::Fit,
            Collapse::Preserve => ListLayout::PreserveBreak,
        },
        ..ListStyle::default()
    }
}

impl NodeFormatter for JsonFormatter {
    fn format_node(
        &self,
        node: &SyntaxNode,
        cx: &FormatCtx<'_>,
    ) -> Result<Option<Doc>, FormatError> {
        let doc = match node.kind() {
            k::OBJECT => delimited_list(cx, node, &list_style(cx.options, true))?,
            k::ARRAY => delimited_list(cx, node, &list_style(cx.options, false))?,
            k::DOCUMENT => document(cx, node)?,
            _ => return Ok(None),
        };
        Ok(Some(doc))
    }
}

/// The value, then comments after it (which are owned by the end-of-file token).
fn document(cx: &FormatCtx<'_>, node: &SyntaxNode) -> Result<Doc, FormatError> {
    core_format::shapes::document(cx, node, 1, None)
}
