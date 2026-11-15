//! SQL formatting: the layout of our own on the parser's tree (`layout`), following sqlfluff's
//! layout rules.

use core_config::shared;
use core_format::{Doc, FormatCtx, FormatError, NodeFormatter};
use core_syntax::SyntaxNode;

include!("schema.generated.rs");

pub struct SqlFormatter;

impl NodeFormatter for SqlFormatter {
    fn format_node(
        &self,
        node: &SyntaxNode,
        cx: &FormatCtx<'_>,
    ) -> Result<Option<Doc>, FormatError> {
        Ok((node.parent().is_none()).then(|| crate::layout::format(node, cx.grammar, cx.options)))
    }
}
