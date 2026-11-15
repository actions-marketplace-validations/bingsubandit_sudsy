//! SQL formatting: the file is printed as written.

use core_format::context::verbatim_text;
use core_format::{Doc, FormatCtx, FormatError, NodeFormatter};
use core_syntax::SyntaxNode;

include!("schema.generated.rs");

pub struct SqlFormatter;

impl NodeFormatter for SqlFormatter {
    fn format_node(
        &self,
        node: &SyntaxNode,
        _cx: &FormatCtx<'_>,
    ) -> Result<Option<Doc>, FormatError> {
        Ok(Some(verbatim_text(&node.text().to_string())))
    }
}
