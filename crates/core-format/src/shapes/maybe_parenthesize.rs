//! Content between delimiters on lines of their own when it does not fit: "parenthesize if the
//! content breaks" (ruff's `maybe_parenthesize`, Prettier's `return (...)`), and the same layout
//! with delimiters that are always there.
//!
//! concept: maybe-parenthesize
//! what: Content between delimiters on lines of its own when it does not fit.
//! tools: ruff maybe_parenthesize, Prettier return (...)
//! reads: option:hanging_clause_kinds, option:hang_operand_kinds

use crate::FormatCtx;
use crate::doc::{self, Doc};
use crate::error::FormatError;
use crate::seq::{children, el};
use core_syntax::roles::un;
use core_syntax::{SyntaxElement, SyntaxKind, SyntaxNode};

/// `open`, the content indented on its own lines, `close`: no group, the group around decides.
pub fn between(open: Doc, inner: Doc, close: Doc) -> Doc {
    doc::concat(vec![
        open,
        doc::indent(doc::concat(vec![doc::softline(), inner])),
        doc::softline(),
        close,
    ])
}

/// [`between`] in a group of its own: the content moves to its own lines when it does not fit.
pub fn enclosed(open: Doc, inner: Doc, close: Doc) -> Doc {
    doc::group(between(open, inner, close))
}

/// The content in parentheses that show only when it breaks.
pub fn maybe_parenthesize(inner: Doc) -> Doc {
    enclosed(
        doc::if_break(doc::lit("("), Doc::Nil),
        inner,
        doc::if_break(doc::lit(")"), Doc::Nil),
    )
}

/// Which operands stay on the line of the head they follow (`(T)x`, `(T)f(a)`): the kinds that
/// always do, the kinds that do unless the head has a kind of `except_head`, and the kinds of
/// call that do when their callee has one of the plain kinds.
pub struct HangPolicy {
    pub always: &'static [SyntaxKind],
    pub plain: &'static [SyntaxKind],
    pub calls: &'static [SyntaxKind],
    pub except_head: &'static [SyntaxKind],
}

/// The children of `node`: a head and its operand (the last child). The operand moves indented
/// onto a line of its own when the whole does not fit, unless it is one of the attached kinds of
/// the policy; only for the kinds of `hang_operand_kinds`. `seq` prints the head's elements.
pub fn hang_operand(
    cx: &FormatCtx<'_>,
    node: &SyntaxNode,
    policy: &HangPolicy,
    seq: &dyn Fn(&[SyntaxElement]) -> Result<Doc, FormatError>,
) -> Result<Doc, FormatError> {
    let els = children(cx, node);
    if !cx.options.list_has(
        core_config::opt::HANG_OPERAND_KINDS,
        cx.grammar.name(node.kind()),
    ) {
        return seq(&els);
    }
    let (Some(head_kind), Some(SyntaxElement::Node(operand))) =
        (node.children().next().map(|c| c.kind()), els.last())
    else {
        return seq(&els);
    };
    let head = seq(&els[..els.len() - 1])?;
    let printed = el(cx, els.last().expect("operand"))?;
    let kind = operand.kind();
    let plain = !policy.except_head.contains(&head_kind)
        && (policy.plain.contains(&kind)
            || (policy.calls.contains(&kind)
                && operand
                    .first_child()
                    .is_some_and(|c| policy.plain.contains(&c.kind()))));
    if policy.always.contains(&kind) || plain {
        return Ok(doc::concat(vec![head, printed]));
    }
    Ok(doc::group(doc::concat(vec![
        head,
        doc::indent(doc::concat(vec![doc::softline(), printed])),
    ])))
}

/// Does a clause of this kind hang (`hanging_clause_kinds`)? Its separator is its own `line`.
pub fn hangs(cx: &FormatCtx<'_>, kind: SyntaxKind) -> bool {
    cx.options.list_has(
        core_config::opt::HANGING_CLAUSE_KINDS,
        cx.grammar.name(kind),
    )
}

/// A clause (`when cond`, `catch ... when (cond)`) on an indented line of its own when the whole
/// does not fit; a condition in parentheses hugs them. After a pattern of
/// `hanging_clause_inline_after` (a parent kind and the kind of its first child) it stays on the
/// line. `seq` prints elements in order; a clause that does not hang is printed so.
pub fn hanging_clause(
    cx: &FormatCtx<'_>,
    node: &SyntaxNode,
    seq: &dyn Fn(&[SyntaxElement]) -> Result<Doc, FormatError>,
) -> Result<Doc, FormatError> {
    let els = children(cx, node);
    if !hangs(cx, node.kind()) {
        return seq(&els);
    }
    let body = match els.as_slice() {
        [kw, open, cond @ SyntaxElement::Node(_), close] => doc::concat(vec![
            seq(&[kw.clone(), open.clone()])?,
            doc::group(doc::concat(vec![
                doc::indent(el(cx, cond)?),
                doc::softline(),
            ])),
            el(cx, close)?,
        ]),
        _ => seq(&els)?,
    };
    let inline = node.parent().is_some_and(|p| {
        p.children().next().is_some_and(|first| {
            cx.options.list_has_pair(
                core_config::opt::HANGING_CLAUSE_INLINE_AFTER,
                cx.grammar.name(p.kind()),
                cx.grammar.name(first.kind()),
            )
        })
    });
    let line = if inline { doc::lit(" ") } else { doc::line() };
    Ok(doc::group(doc::indent(doc::concat(vec![line, body]))))
}

/// What follows a keyword that takes an expression (`return a`, `throw a`): a blank and the
/// argument. A binary chain (`#binary-chain`) goes in parentheses on lines of its own when it does
/// not fit; a sequence (`#sequence`) always has its parentheses, on one line.
pub fn keyword_argument(cx: &FormatCtx<'_>, arg: &SyntaxNode, printed: Doc) -> Doc {
    let g = cx.grammar;
    let sequence = g.has_tag(arg.kind(), "sequence");
    if !sequence && !g.has_tag(arg.kind(), "binary-chain") {
        return doc::concat(vec![doc::lit(" "), printed]);
    }
    let paren = |text| {
        if sequence {
            doc::lit(text)
        } else {
            doc::if_break(doc::lit(text), Doc::Nil)
        }
    };
    doc::group(doc::concat(vec![
        doc::lit(" "),
        between(paren("("), printed, paren(")")),
    ]))
}

/// The test between the parentheses of a compound head (`if (` `)`): `soft` lets it move onto
/// lines of its own when it does not fit, except a negated chain (`!(a && b)`), which keeps the
/// head's line and breaks inside its own parentheses.
pub fn head_test(cx: &FormatCtx<'_>, test: &SyntaxNode, printed: Doc, soft: bool) -> Doc {
    if soft && !negated_chain(cx, test) {
        enclosed(Doc::Nil, printed, Doc::Nil)
    } else {
        printed
    }
}

/// `!(a && b)`: a negation of a binary chain.
fn negated_chain(cx: &FormatCtx<'_>, test: &SyntaxNode) -> bool {
    let g = cx.grammar;
    g.has_tag(test.kind(), "unary")
        && g.slot(test, "operator")
            .and_then(|o| o.into_token())
            .is_some_and(|o| {
                // (a comment in front of the `!` is part of the test's text)
                o.text() == "!"
                    && test
                        .descendants_with_tokens()
                        .filter_map(|e| e.into_token())
                        .find(|t| {
                            !g.has_tag(t.kind(), "whitespace") && !g.has_tag(t.kind(), "newline")
                        })
                        == Some(o)
            })
        && g.slot(test, "operand")
            .and_then(|e| e.into_node())
            .is_some_and(|n| g.has_tag(un(&n).kind(), "binary-chain"))
}
