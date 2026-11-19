//! Tabs in a block that a line-oriented formatter rewrites: a tab changes how the block is read
//! (its width depends on the column), so the block is kept as written, except where the tab only
//! indents a list marker and the tree already tells the nesting.
//!
//! concept: tab-keep
//! what: Which tabs make a block verbatim: all but those before a list marker of a plain list.
//! tools: Prettier markdown

use core_syntax::{SyntaxElement, SyntaxKind};

/// What a language says about its lists: the characters a list item starts with (after the
/// indentation, one of `bullets`, or digits followed by one of `ordered`, then a blank or the end
/// of the line), the node kind of a list, and the kinds that make a list verbatim when inside it.
#[derive(Clone, Copy, Debug)]
pub struct ListFacts {
    pub bullets: &'static [char],
    pub ordered: &'static [char],
    pub list: SyntaxKind,
    pub verbatim: &'static [SyntaxKind],
}

impl ListFacts {
    fn starts(&self, body: &str) -> bool {
        let digits = body.bytes().take_while(u8::is_ascii_digit).count();
        let rest = if digits > 0 && digits < 10 {
            body[digits..].strip_prefix(self.ordered)
        } else {
            body.strip_prefix(self.bullets)
        };
        rest.is_some_and(|r| r.is_empty() || r.starts_with([' ', '\t']))
    }

    fn plain(&self, e: &SyntaxElement) -> bool {
        matches!(e, SyntaxElement::Node(n) if n.kind() == self.list
            && !n.descendants().any(|d| self.verbatim.contains(&d.kind())))
    }
}

/// Does a tab in `text` (the source of `e`) change what the formatter writes? A tab in the
/// indentation of a line that starts a list item does not when `e` is a list without verbatim
/// content in it; any other tab does.
pub fn tab_changes_layout(e: &SyntaxElement, text: &str, facts: &ListFacts) -> bool {
    let plain = facts.plain(e);
    text.lines().any(|l| {
        let body = l.trim_start_matches([' ', '\t']);
        l.contains('\t') && !(plain && facts.starts(body) && !body.contains('\t'))
    })
}
