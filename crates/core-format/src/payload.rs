//! A directive whose payload the language parsed (the parameters and the body of a `#define`, the
//! condition of an `#if`): the header as written with its blanks collapsed, the parameters and the
//! body laid out as the code they are. A body of several lines goes below the header, one level in,
//! and every line but the last ends in a backslash; a body of one line stays on the header's line.
//!
//! concept: directive-payload-layout
//! what: A directive's parsed payload is laid out as code, its lines joined with aligned backslashes.
//! tools: clang-format

use crate::FormatCtx;
use crate::doc::{self, Doc};
use crate::error::FormatError;
use crate::printer;
use crate::relay::{self, Place};
use crate::seq::{first, kids};
use crate::shapes::flow::Flow;
use crate::shapes::statement_sequence;
use core_syntax::{SyntaxElement, SyntaxKind, SyntaxNode};

/// `node` laid out by its payload, or `None` when it has none the shape can lay out (the caller
/// prints the text as written). `head` are the kinds of nodes that belong to the header (the
/// parameter list), `tight_hash` drops the blanks after the `#`, `place` puts the
/// backslashes.
pub(crate) fn layout(
    cx: &FormatCtx<'_>,
    node: &SyntaxNode,
    flow: &Flow<'_>,
    head: &[SyntaxKind],
    tight_hash: bool,
    place: Place,
) -> Option<Result<Doc, FormatError>> {
    let els = kids(cx, node);
    let at = els
        .iter()
        .position(|e| matches!(e, SyntaxElement::Node(_)))?;
    let tokens = els[..at]
        .iter()
        .all(|e| matches!(e, SyntaxElement::Token(_)));
    let params =
        usize::from(matches!(&els[at], SyntaxElement::Node(n) if head.contains(&n.kind())));
    let body = &els[at + params..];
    let items: Vec<SyntaxNode> = body
        .iter()
        .filter_map(|e| match e {
            SyntaxElement::Node(n) => Some(n.clone()),
            SyntaxElement::Token(_) => None,
        })
        .collect();
    if !tokens || items.len() != body.len() || at == 0 || commented(cx, node) {
        return None;
    }
    let (Some(start), Some(last)) = (first(cx, &els[0]), first(cx, &els[at - 1])) else {
        return None;
    };
    let text = node.text().to_string();
    let from = usize::from(start.text_range().start() - node.text_range().start());
    let to = usize::from(last.text_range().end() - node.text_range().start());
    let head_text = relay::relay(&text[from..to], Place::NONE, None);
    let head_text = match head_text.strip_prefix('#') {
        Some(rest) if tight_hash => format!("#{}", rest.trim_start_matches([' ', '\t'])),
        _ => head_text,
    };
    Some((|| {
        let mut parts = vec![doc::text(head_text)];
        if params == 1 {
            parts.push(flow.seq(cx, &els[at..=at])?);
        }
        if !items.is_empty() {
            let lines = statement_sequence(cx, &items, 0)?;
            // (labels of a `switch` stand one level in, the lines below them one more)
            let lines = if items.iter().any(|n| cx.grammar.has_tag(n.kind(), "case")) {
                doc::indent(lines)
            } else {
                lines
            };
            let several = several_lines(cx, &items);
            // (the comment behind a one-line body does not move it)
            let lines = if several {
                lines
            } else {
                lines.without_comment_widths()
            };
            if several {
                parts.push(doc::indent(doc::concat(vec![doc::hardline(), lines])));
            } else {
                // (a body goes below a header that names something when the line does not fit; a
                // condition stays on the line of its directive)
                let head = doc::concat(std::mem::take(&mut parts));
                if at > 2 {
                    parts.push(doc::group(doc::concat(vec![
                        head,
                        doc::indent(doc::concat(vec![doc::line(), lines])),
                    ])));
                } else {
                    parts.push(doc::concat(vec![head, doc::text(" ".to_string()), lines]));
                }
            }
        }
        let whole = doc::concat(parts);
        // (a directive of one line has the whole width; the lines of a longer one need room for their backslash)
        let mut printed = print_at(cx, whole.clone(), place.width);
        if printed.contains('\n') {
            printed = print_at(cx, whole, place.width.saturating_sub(2));
        }
        let lead = cx.leading_comments_doc_before_directive(&start);
        Ok(doc::concat(vec![
            lead,
            doc::line_start(relay::escape_lines(&printed, place)),
        ]))
    })())
}

/// Is the body more than one unwrapped line, as the reference tool reads it: more than one
/// statement, or a block?
fn several_lines(cx: &FormatCtx<'_>, items: &[SyntaxNode]) -> bool {
    let mut statements = 0;
    for n in items.iter().flat_map(|n| n.descendants()) {
        if cx.grammar.has_tag(n.kind(), "block") {
            return true;
        }
        statements += usize::from(cx.grammar.has_tag(n.kind(), "statement"));
    }
    statements > 1
}

/// Does the directive hold a comment the backslashes of its lines would break: a block comment
/// over several lines, or a line comment before more code?
fn commented(cx: &FormatCtx<'_>, node: &SyntaxNode) -> bool {
    let tokens: Vec<_> = node
        .descendants_with_tokens()
        .filter_map(|e| e.into_token())
        .collect();
    let code = |t: &core_syntax::SyntaxToken| !cx.grammar.is_trivia(t.kind());
    let (Some(a), Some(z)) = (tokens.iter().position(code), tokens.iter().rposition(code)) else {
        return false;
    };
    tokens.iter().enumerate().any(|(i, t)| {
        cx.grammar.is_trivia(t.kind())
            && match t.text() {
                s if s.starts_with("//") => i > a && i < z,
                s if s.starts_with("/*") => i > a && s.contains('\n'),
                _ => false,
            }
    })
}

/// The document printed at `width` columns.
fn print_at(cx: &FormatCtx<'_>, mut doc: Doc, width: usize) -> String {
    let mut opts = crate::print_options(cx.options);
    opts.line_width = width;
    printer::print(&mut doc, &opts)
}
