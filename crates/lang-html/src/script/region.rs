//! Where the script of a component is: the root `<script>` elements. `<script>` and
//! `<script setup>` are one JavaScript program (as the Vue compiler and `vue-eslint-parser` read
//! them), so the region runs from the first block to the last and the markup between them is
//! carved out.

use crate::dom;
use crate::generated::kind as k;
use core_syntax::{Embedded, SyntaxNode, SyntaxNodeExt, TextRange};

pub(super) struct Block<'a> {
    pub content: TextRange,
    pub setup: bool,
    pub lang: Option<&'a str>,
}

/// The `<script>` elements that are children of the root, in order.
pub(super) fn blocks<'a>(
    root: &SyntaxNode,
    text: &'a str,
) -> impl Iterator<Item = Block<'a>> + use<'a> {
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
            lang: attrs
                .iter()
                .find(|a| a.name == "lang")
                .and_then(|a| a.value),
        })
    })
}

/// The script of the file as one region of JavaScript or TypeScript, which the host reads
/// itself (the language's own rules do not run on it).
pub fn find(root: &SyntaxNode, text: &str) -> Vec<Embedded> {
    let mut blocks = blocks(root, text);
    let Some(first) = blocks.next() else {
        return Vec::new();
    };
    let (mut last, mut lang, mut carved) = (first.content, first.lang, Vec::new());
    for b in blocks {
        carved.push(TextRange::new(last.end(), b.content.start()));
        last = b.content;
        lang = lang.or(b.lang);
    }
    let language = match lang {
        None | Some("js") => "js",
        Some(l @ ("ts" | "tsx" | "jsx")) => l,
        Some(_) => return Vec::new(),
    };
    let mut region = Embedded::new(
        language,
        TextRange::new(first.content.start(), last.end()),
        carved,
    );
    region.lint = false;
    vec![region]
}
