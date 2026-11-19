//! The body of a lambda or a switch rule after its arrow: a block stays on the line of the head
//! (or is left to the caller), any other body goes on the next line, indented, when it does not
//! fit; a comment after the arrow puts the body on the next line.
//!
//! concept: assignment-layout
//! what: The head, the arrow and the body of a lambda or a switch rule.
//! tools: google-java-format lambdas and switch rules, C# lambda bodies

use super::Doc;
use crate::FormatCtx;
use crate::doc;
use crate::error::FormatError;
use crate::seq;
use core_syntax::{SyntaxElement, SyntaxKind, SyntaxNode};

/// The options of [`arrow_body`].
pub struct ArrowStyle {
    /// Indentation levels the body moves in below the head.
    pub levels: usize,
    /// The line break in front of the body sits in a group of its own: the body stays on the
    /// line of the head when it fits there, whatever the head does.
    pub own_group: bool,
    /// A block body follows the head on its line; else a block body is the caller's (`None`).
    pub hug_block: bool,
}

/// `head arrow body` (`node`'s children; `arrow` the kind of the arrow token): `print` lays out a
/// run of the children. `None`: the node has no arrow or no body, or has a block body that the
/// style leaves to the caller; the caller lays it out its own way.
pub fn arrow_body(
    cx: &FormatCtx<'_>,
    node: &SyntaxNode,
    print: &dyn Fn(&[SyntaxElement]) -> Result<Doc, FormatError>,
    arrow: SyntaxKind,
    style: &ArrowStyle,
) -> Result<Option<Doc>, FormatError> {
    let els = seq::kids(cx, node);
    let at = els.iter().position(|e| e.kind() == arrow);
    let Some((at, body)) = at
        .map(|i| (i, &els[i + 1..]))
        .filter(|(_, b)| !b.is_empty())
    else {
        return Ok(None);
    };
    let head = print(&els[..=at])?;
    if cx.grammar.has_tag(body[0].kind(), "block") {
        return if style.hug_block {
            Ok(Some(doc::concat(vec![head, doc::lit(" "), print(body)?])))
        } else {
            Ok(None)
        };
    }
    let broken = seq::last(cx, &els[at]).is_some_and(|t| seq::line_comment_after(cx, &t));
    let sep = if broken { doc::hardline() } else { doc::line() };
    let moved = doc::concat(vec![sep, print(body)?]);
    let moved = if style.own_group {
        doc::group(moved)
    } else {
        moved
    };
    let body = (0..style.levels).fold(moved, |d, _| doc::indent(d));
    Ok(Some(doc::group(doc::concat(vec![head, body]))))
}
