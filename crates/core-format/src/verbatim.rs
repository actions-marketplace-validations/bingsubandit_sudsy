//! A formatter that prints the file as written, for a language whose own tool formats it (gofmt) or that has nothing to lay out yet.
//!
//! concept: verbatim-file
//! what: A file printed as written; only the final newline and line ending options apply, and a file that ends in one of the listed token kinds keeps its end.
//! tools: gofmt (shipped with Go, not reimplemented)
//! reads: option:final_newline

use crate::{Doc, FormatCtx, FormatError, NodeFormatter, doc};
use core_syntax::{SyntaxKind, SyntaxNode};

/// The root prints as its source text; the engine's own checks (final newline, line endings) still apply.
pub struct VerbatimFormatter {
    /// Token kinds that end the file as data (Ruby's text after `__END__`): a file whose last
    /// token that is not blank is one of them keeps its end byte for byte.
    pub exact_end: &'static [SyntaxKind],
}

impl NodeFormatter for VerbatimFormatter {
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

    fn exact_end(&self, root: &SyntaxNode) -> bool {
        let mut last = root.last_token();
        while let Some(t) = last {
            if self.exact_end.contains(&t.kind()) {
                return true;
            }
            if !t.text().trim().is_empty() {
                return false;
            }
            last = t.prev_token();
        }
        false
    }
}
