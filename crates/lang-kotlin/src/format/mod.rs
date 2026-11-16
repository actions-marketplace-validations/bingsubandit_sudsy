//! Kotlin formatting: the layout of ktlint's `ktlint_official` code style (docs/formatting/kotlin.md).
//! The file is laid out as one piece (`layout`): `plan` decides line breaks and blanks, `indent`
//! the indentation of each line, `space` the blanks inside a line.

use core_config::shared;
use core_format::{Doc, FormatCtx, FormatError, NodeFormatter};
use core_syntax::SyntaxNode;

mod blank;
mod indent;
mod layout;
mod plan;
mod space;
mod src;

include!("../schema.generated.rs");

pub struct KotlinFormatter;

impl NodeFormatter for KotlinFormatter {
    fn format_node(
        &self,
        node: &SyntaxNode,
        cx: &FormatCtx<'_>,
    ) -> Result<Option<Doc>, FormatError> {
        Ok(node.parent().is_none().then(|| layout::file(node, cx)))
    }
}
