//! SQL formatting: the layout of our own on the parser's tree (`layout`), following sqlfluff's
//! layout rules.
//!
//! concept: none (glue: formatter dispatch)

use core_config::shared;
use core_format::{Doc, FormatCtx, FormatError, NodeFormatter};
use core_syntax::{SyntaxElement, SyntaxNode};
use std::borrow::Cow;

include!("schema.generated.rs");

pub struct SqlFormatter;

impl NodeFormatter for SqlFormatter {
    fn rows(&self) -> &'static str {
        crate::layout::ROWS.rows()
    }

    fn format_node(
        &self,
        node: &SyntaxNode,
        cx: &FormatCtx<'_>,
    ) -> Result<Option<Doc>, FormatError> {
        Ok((node.parent().is_none()).then(|| crate::layout::format(node, cx.grammar, cx.options)))
    }

    /// The words whose case is set compare lowercase.
    fn value<'a>(&self, item: &'a SyntaxElement) -> Cow<'a, str> {
        match item {
            SyntaxElement::Token(t) => Cow::Owned(t.text().to_lowercase()),
            SyntaxElement::Node(n) => Cow::Owned(n.text().to_string()),
        }
    }
}
