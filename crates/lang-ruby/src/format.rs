//! Ruby formatting (see docs/formatting/ruby.md).

use crate::generated::kind as k;
use core_config::shared;
use core_format::{Doc, FormatCtx, FormatError, NodeFormatter, doc};
use core_syntax::SyntaxNode;

include!("schema.generated.rs");

pub struct RubyFormatter;

impl NodeFormatter for RubyFormatter {
    fn format_node(
        &self,
        node: &SyntaxNode,
        _: &FormatCtx<'_>,
    ) -> Result<Option<Doc>, FormatError> {
        Ok(node
            .parent()
            .is_none()
            .then(|| doc::verbatim(node.text().to_string())))
    }

    /// The text after `__END__` is data: a file that ends in it keeps its end.
    fn exact_end(&self, root: &SyntaxNode) -> bool {
        let mut last = root.last_token();
        while let Some(t) = last {
            match t.kind() {
                k::DATA => return true,
                k::KEYWORD if t.text().starts_with("__END__") => return true,
                k::EOF | k::WHITESPACE | k::NEWLINE | k::COMMENT => last = t.prev_token(),
                _ => return false,
            }
        }
        false
    }
}
