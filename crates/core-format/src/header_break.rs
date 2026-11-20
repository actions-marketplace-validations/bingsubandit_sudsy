//! The line break after a header in front of a declaration (`template <...>`): always, never, or
//! only when the declaration, up to the `{` of its body, does not fit on the header's line.
//!
//! concept: header-line-break
//! what: A declaration's header ends its line always, never, or when the declaration does not fit on it.
//! tools: clang-format BreakTemplateDeclarations

use crate::FormatError;
use crate::doc::{self, Doc};
use core_syntax::{SyntaxElement, SyntaxNode};

/// Does the source write the declaration over several lines before its `{` (it wraps), or a line
/// comment in it?
fn wraps_before_body(rest: &[SyntaxElement]) -> bool {
    // `seen`: text came; `broke`: a line break came after it
    let (mut seen, mut broke) = (false, false);
    let mut token = |s: &str| -> Option<bool> {
        if s.contains('{') {
            return Some(false);
        }
        if s.starts_with("//") || (broke && !s.trim().is_empty()) {
            return Some(true);
        }
        if s.trim().is_empty() {
            broke |= seen && s.contains('\n');
        } else {
            seen = true;
            broke = s.trim_end().contains('\n');
        }
        None
    };
    for e in rest {
        match e {
            SyntaxElement::Token(t) => {
                if let Some(r) = token(t.text()) {
                    return r;
                }
            }
            SyntaxElement::Node(n) => {
                for t in n.descendants_with_tokens().filter_map(|d| d.into_token()) {
                    if let Some(r) = token(t.text()) {
                        return r;
                    }
                }
            }
        }
    }
    false
}

/// The elements of `node`: the header `els[..=head]` and the declaration after it. `mode` is
/// `yes`, `no` or `multiline`; with `multiline` the declaration joins the header's line when the
/// two, as printed, fit up to the declaration's first forced break (the `{` of its body); a
/// declaration that is itself headed (`template <..> template <..> ...`) or a header with a comment after it or over several lines never joins. `render`
/// lays out a run of elements.
pub fn header_break(
    node: &SyntaxNode,
    els: &[SyntaxElement],
    head: usize,
    mode: &str,
    commented: bool,
    render: &dyn Fn(&[SyntaxElement]) -> Result<Doc, FormatError>,
) -> Result<Doc, FormatError> {
    let (header, rest) = els.split_at(head + 1);
    let header_doc = render(header)?;
    let Some(first_rest) = rest.first() else {
        return Ok(header_doc);
    };
    let rest_doc = render(rest)?;
    match mode {
        "yes" => return Ok(doc::concat(vec![header_doc, doc::hardline(), rest_doc])),
        "no" => return Ok(doc::concat(vec![header_doc, doc::lit(" "), rest_doc])),
        _ => {}
    }
    let nested = first_rest
        .as_node()
        .is_some_and(|n| n.kind() == node.kind())
        || node.parent().is_some_and(|p| p.kind() == node.kind());
    // The header and the declaration up to its first forced break (the `{` of a body), as printed.
    // (a header that spans lines, as the source has it, ends its line)
    let multiline = els[head]
        .as_node()
        .is_some_and(|n| n.text().to_string().trim_end().contains('\n'))
        || wraps_before_body(rest);
    let width = if nested || commented || multiline {
        10_000
    } else {
        header_doc.tail_width() + 1 + rest_doc.width_to_hardline()
    };
    let rest = doc::shared(rest_doc);
    Ok(doc::concat(vec![
        header_doc,
        doc::fits_below(
            0,
            width,
            doc::concat(vec![doc::lit(" "), rest.clone()]),
            doc::concat(vec![doc::hardline(), rest]),
        ),
    ]))
}
