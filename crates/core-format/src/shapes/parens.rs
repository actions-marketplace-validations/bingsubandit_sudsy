//! Parentheses: a node tagged `#parenthesized` is `(`, one expression and `)`. A [`ParenPolicy`]
//! says whether a pair that is not needed stays and how its content may break; which pairs are
//! needed is the language's judgement ([`ParenHooks::needs`]), for operators decided by the
//! grammar's `precedence` lines ([`binary_needs`]).
//!
//! concept: parenthesized-expression
//! what: Needed and unneeded parentheses and the layout of their content.
//! tools: Prettier needs-parens, ruff parenthesized, ktlint no-unnecessary-parentheses
//! reads: tag:parenthesized

use super::binary_chain::flattens;
use crate::FormatCtx;
use crate::comments::{self, CommentKind};
use crate::doc::{self, Doc};
use crate::error::FormatError;
use core_syntax::{Grammar, SyntaxElement, SyntaxNode, SyntaxToken};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Need {
    Yes,
    No,
    /// A kind the language does not decide on: the source's parentheses stay.
    Keep,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Unneeded {
    /// Every pair of the source stays.
    Keep,
    /// A pair the hooks do not need is left out; a node that needs one the source lacks gets it.
    Drop,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ParenLayout {
    /// `(expr)`: the layout around decides where it breaks.
    Flat,
    /// The content moves to its own indented line when it does not fit.
    Group,
}

pub struct ParenPolicy {
    pub layout: ParenLayout,
    pub unneeded: Unneeded,
}

/// What only the language can judge; every method has a default.
pub trait ParenHooks {
    /// Does `node` (not a parenthesized expression) need parentheses where its parent holds it?
    fn needs(&self, _cx: &FormatCtx<'_>, _node: &SyntaxNode) -> Need {
        Need::Keep
    }
    /// May `node` get parentheses when the source has none (`needs` is `Yes`)?
    fn addable(&self, _cx: &FormatCtx<'_>, _node: &SyntaxNode) -> bool {
        false
    }
    /// Does something besides a comment inside the pair tie it to its expression (a type cast)?
    fn tied(&self, _cx: &FormatCtx<'_>, _open: &SyntaxToken) -> bool {
        false
    }
}

/// A language that decides nothing: its pairs stay as written.
pub struct NoParenHooks;

impl ParenHooks for NoParenHooks {}

/// The expression inside any number of parentheses.
pub fn unwrap(g: &Grammar, n: &SyntaxNode) -> SyntaxNode {
    let mut n = n.clone();
    while g.has_tag(n.kind(), "parenthesized") {
        match n.children().next() {
            Some(c) => n = c,
            None => break,
        }
    }
    n
}

fn is_paren(g: &Grammar, n: &SyntaxNode) -> bool {
    g.has_tag(n.kind(), "parenthesized")
}

fn ends(cx: &FormatCtx<'_>, node: &SyntaxNode) -> Option<(SyntaxToken, SyntaxToken)> {
    let mut sig = node
        .children_with_tokens()
        .filter(|e| !cx.grammar.is_trivia(e.kind()));
    let open = sig.next()?.into_token()?;
    let close = sig.last()?.into_token()?;
    Some((open, close))
}

/// Does a comment tie the source's pair to its expression: one inside the pair (not a line
/// comment after `(`, which moves out with the pair), or one the hooks name.
fn tied(
    cx: &FormatCtx<'_>,
    open: &SyntaxToken,
    close: &SyntaxToken,
    hooks: &dyn ParenHooks,
) -> bool {
    hooks.tied(cx, open)
        || comments::trailing_comments(cx.grammar, open)
            .iter()
            .any(|c| c.kind != CommentKind::Line)
        || !comments::leading_comments(cx.grammar, close).is_empty()
}

/// Is the pair `node` kept in the output?
fn kept(
    cx: &FormatCtx<'_>,
    node: &SyntaxNode,
    open: &SyntaxToken,
    close: &SyntaxToken,
    policy: &ParenPolicy,
    hooks: &dyn ParenHooks,
) -> bool {
    policy.unneeded == Unneeded::Keep
        || tied(cx, open, close, hooks)
        || (!node.parent().is_some_and(|p| is_paren(cx.grammar, &p))
            && hooks.needs(cx, &unwrap(cx.grammar, node)) != Need::No)
}

/// `( expr )` with its comments: printed as written when kept, otherwise the expression alone.
pub fn parenthesized(
    cx: &FormatCtx<'_>,
    node: &SyntaxNode,
    policy: &ParenPolicy,
    hooks: &dyn ParenHooks,
) -> Result<Doc, FormatError> {
    let shape = || FormatError::Shape("parentheses".into());
    let (open, close) = ends(cx, node).ok_or_else(shape)?;
    let child = node
        .children()
        .find(|c| !cx.grammar.is_trivia(c.kind()))
        .ok_or_else(shape)?;
    if !kept(cx, node, &open, &close, policy, hooks) {
        return Ok(doc::concat(vec![
            cx.leading_comments_doc(&open),
            cx.trailing_comments_doc(&open),
            cx.format(&child)?,
            cx.trailing_comments_doc(&close),
        ]));
    }
    Ok(match policy.layout {
        ParenLayout::Flat => {
            doc::concat(vec![cx.token(&open), cx.format(&child)?, cx.token(&close)])
        }
        ParenLayout::Group => paren_group(cx, &open, || cx.format(&child), &close)?,
    })
}

/// `( inner )` where `inner` may move to its own, indented line. A comment after the closing
/// parenthesis does not make the group break. `inner` runs after `(` is printed, so the comments
/// are taken in source order.
pub fn paren_group(
    cx: &FormatCtx<'_>,
    open: &SyntaxToken,
    inner: impl FnOnce() -> Result<Doc, FormatError>,
    close: &SyntaxToken,
) -> Result<Doc, FormatError> {
    let open = cx.token(open);
    Ok(doc::concat(vec![
        doc::group(doc::concat(vec![
            open,
            doc::indent(doc::concat(vec![doc::softline(), inner()?])),
            doc::softline(),
            cx.leading_comments_doc(close),
            cx.token_text(close),
        ])),
        cx.trailing_comments_doc(close),
    ]))
}

/// Is `close` the `)` of a pair that is left out of the output?
pub fn is_dropped_close(
    cx: &FormatCtx<'_>,
    close: &SyntaxToken,
    policy: &ParenPolicy,
    hooks: &dyn ParenHooks,
) -> bool {
    let Some(paren) = close.parent().filter(|p| is_paren(cx.grammar, p)) else {
        return false;
    };
    ends(cx, &paren)
        .is_some_and(|(open, c)| c == *close && !kept(cx, &paren, &open, close, policy, hooks))
}

/// `doc` of `node` in the parentheses it needs when the source has none.
pub fn wrap(
    cx: &FormatCtx<'_>,
    node: &SyntaxNode,
    doc: Doc,
    policy: &ParenPolicy,
    hooks: &dyn ParenHooks,
) -> Doc {
    if policy.unneeded == Unneeded::Keep
        || node.parent().is_none_or(|p| is_paren(cx.grammar, &p))
        || !hooks.addable(cx, node)
        || hooks.needs(cx, node) != Need::Yes
    {
        return doc;
    }
    doc::concat(vec![doc::lit("("), doc, doc::lit(")")])
}

/// The first character the printed `node` starts with (a kept or added parenthesis counts).
pub fn first_char(
    cx: &FormatCtx<'_>,
    node: &SyntaxNode,
    policy: &ParenPolicy,
    hooks: &dyn ParenHooks,
) -> Option<char> {
    let g = cx.grammar;
    let mut n = node.clone();
    loop {
        if is_paren(g, &n) {
            let kept = ends(cx, &n).is_some_and(|(o, c)| kept(cx, &n, &o, &c, policy, hooks));
            if kept {
                return Some('(');
            }
            n = n.first_child()?;
            continue;
        }
        if n.parent().is_some_and(|p| !is_paren(g, &p)) && hooks.needs(cx, &n) == Need::Yes {
            return Some('(');
        }
        match n.children_with_tokens().find(|e| !g.is_trivia(e.kind()))? {
            SyntaxElement::Node(c) => n = c,
            SyntaxElement::Token(t) => return t.text().chars().next(),
        }
    }
}

/// Does the operator `op` of a node need parentheses inside the operator `parent_op` (`first`:
/// on its left)? The grammar's `precedence` lines decide; `clarify` adds the pairs that make
/// mixed arithmetic and bit operations readable (`a % b + c` stays `(a % b) + c`, an operand of
/// a bitwise operator stays in parentheses).
pub fn binary_needs(g: &Grammar, parent_op: &str, op: &str, first: bool, clarify: bool) -> Need {
    let level = |o: &str| g.precedence(o).map_or(0, |p| p.level);
    let (np, pp) = (level(op), level(parent_op));
    let joins = flattens(g, parent_op, op);
    // The left side of a chain of one precedence prints without its own parentheses.
    if first && pp == np && joins {
        return Need::No;
    }
    let yes = pp > np
        || (pp == np && (!first || !joins))
        || (clarify
            && ((pp < np && op == "%" && matches!(parent_op, "+" | "-"))
                || g.operator_kind(parent_op) == Some("bitwise")));
    if yes { Need::Yes } else { Need::No }
}

/// Does a comment after the closing parenthesis of `paren` belong to its statement's line: the
/// parent of the pair is a kind of the option `paren_line_end_parents` (`await (x)`, `del (x)`)
/// and the pair is the last token of its statement.
pub fn ends_statement_line(cx: &FormatCtx<'_>, paren: &SyntaxNode) -> bool {
    let g = cx.grammar;
    let listed = paren.parent().is_some_and(|p| {
        cx.options
            .list_has("paren_line_end_parents", g.name(p.kind()))
    });
    listed
        && cx.last_token(paren).is_some_and(|t| {
            t.parent_ancestors()
                .find(|a| g.has_tag(a.kind(), "statement"))
                .is_some_and(|stmt| cx.last_token(&stmt).as_ref() == Some(&t))
        })
}
