//! SQL formatting: sqruff's fixer rewrites the source (`fix`); the layout step then prints the
//! result as written. Keywords, identifiers, functions and types may change case, so the safety
//! check compares tokens lowercase.

use core_config::{ResolvedOptions, shared};
use core_format::{Doc, FormatCtx, FormatError, NodeFormatter};
use core_syntax::{Grammar, SyntaxElement, SyntaxNode};
use std::borrow::Cow;

include!("schema.generated.rs");

pub struct SqlFormatter;

impl NodeFormatter for SqlFormatter {
    /// sqruff's fixer is not a fixed point after one run on some statements (a line break it adds
    /// lets another rule break the next): the rewrite repeats until the text is stable.
    fn rewrite_rounds(&self) -> usize {
        4
    }

    fn rewrite_source(
        &self,
        tree: &SyntaxNode,
        text: &str,
        grammar: &Grammar,
        options: &ResolvedOptions,
    ) -> Option<String> {
        crate::fix::rewrite(self, tree, text, grammar, options)
    }

    fn value<'a>(&self, item: &'a SyntaxElement) -> Cow<'a, str> {
        match item {
            SyntaxElement::Token(t) => Cow::Owned(t.text().to_lowercase()),
            SyntaxElement::Node(n) => Cow::Owned(n.text().to_string()),
        }
    }

    fn format_node(
        &self,
        node: &SyntaxNode,
        cx: &FormatCtx<'_>,
    ) -> Result<Option<Doc>, FormatError> {
        Ok((node.parent().is_none()).then(|| cx.verbatim_node(node)))
    }
}
