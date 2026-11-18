//! The root `<script>` elements of a component (the embedded region is the `embed` row of
//! `grammars/html.grammar`).
//!
//! concept: embedded-code

use crate::dom;
use crate::generated::kind as k;
use core_syntax::{SyntaxNode, SyntaxNodeExt, TextRange};

pub(super) struct Block {
    pub content: TextRange,
    pub setup: bool,
}

/// The `<script>` elements that are children of the root, in order.
pub(super) fn blocks<'a>(
    root: &SyntaxNode,
    text: &'a str,
) -> impl Iterator<Item = Block> + use<'a> {
    root.children().filter_map(move |el| {
        let tag = el.child(k::START_TAG).filter(|_| el.kind() == k::ELEMENT)?;
        let name = tag.token(k::TAG_NAME)?;
        if !text[name.text_range()].eq_ignore_ascii_case("script") {
            return None;
        }
        let content = el.child(k::CHUNK)?.text_range();
        let attrs = dom::attrs(&tag, text);
        Some(Block {
            content,
            setup: attrs.iter().any(|a| a.name == "setup"),
        })
    })
}
