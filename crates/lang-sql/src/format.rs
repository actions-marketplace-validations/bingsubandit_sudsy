//! SQL formatting: the file prints as written until the layout of our own lands (sqruff is the
//! parser only).

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
        Ok((node.parent().is_none()).then(|| cx.verbatim_node(node)))
    }
}
