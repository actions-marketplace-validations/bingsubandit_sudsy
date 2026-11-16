//! Kotlin formatting (ktlint's layout, see docs/formatting/kotlin.md).

use core_config::shared;
use core_format::{Doc, FormatCtx, FormatError, NodeFormatter};
use core_syntax::SyntaxNode;

include!("schema.generated.rs");

pub struct KotlinFormatter;

impl NodeFormatter for KotlinFormatter {
    fn format_node(
        &self,
        node: &SyntaxNode,
        cx: &FormatCtx<'_>,
    ) -> Result<Option<Doc>, FormatError> {
        Ok(node.parent().is_none().then(|| cx.verbatim_node(node)))
    }
}
