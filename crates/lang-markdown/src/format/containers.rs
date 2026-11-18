//! Containers (quotes, list items) and the source lines of blocks.

use super::*;

impl Render<'_> {
    /// The containers of the source, through the grammar's `container` rows.
    pub(super) fn cs(&self) -> Containers<'_> {
        Containers {
            grammar: core_syntax::grammars::MARKDOWN.get(),
            src: self.src,
        }
    }

    /// The blocks of a container, as elements.
    pub(super) fn block_elems(parent: &SyntaxNode) -> Vec<SyntaxElement> {
        parent
            .children_with_tokens()
            .filter(|e| match e {
                SyntaxElement::Node(_) => true,
                SyntaxElement::Token(t) => [
                    k::INDENTED_CODE,
                    k::THEMATIC_BREAK,
                    k::LINK_DEF,
                    k::FRONT_MATTER,
                ]
                .contains(&t.kind()),
            })
            .collect()
    }

    /// The element as written, minus the prefixes of the containers around it.
    pub(super) fn verbatim(&self, e: &SyntaxElement) -> Vec<String> {
        let (Some(f), Some(l)) = elem_ends(e) else {
            return Vec::new();
        };
        let chain = match e {
            SyntaxElement::Node(n) => self.cs().chain(n),
            SyntaxElement::Token(t) => self.cs().chain_of_token(t),
        };
        let (mut from, mut to) = (start(&f), end(&l));
        if let SyntaxElement::Node(n) = e
            && n.kind() == k::HEADING
        {
            // The markers are trivia outside the text: an ATX heading's `#`s, a setext underline.
            if atx_level(self.src, n).is_some() {
                from = self.src[..from].trim_end_matches([' ', '\t', '#']).len();
            } else {
                to += self.src[to..].find('\n').map_or(0, |i| i + 1);
                to += self.src[to..].find('\n').unwrap_or(self.src.len() - to);
            }
        }
        // A top-level block keeps the indentation of its first line.
        let line = self.cs().line_start(from);
        if chain.is_empty() && self.src[line..from].trim().is_empty() {
            from = line;
        }
        raw_lines(&self.src[from..to], &chain)
    }
}
