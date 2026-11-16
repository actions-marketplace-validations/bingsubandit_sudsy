//! Comments around statements: those at the end of a block, the ones a block has already
//! printed, `# fmt: off` and `# fmt: skip` marks, and the comment text normalized before
//! formatting.

use super::util::*;
use crate::generated::kind as k;
use core_format::FormatCtx;
use core_format::comments::{self, Comment};
use core_format::context::leading_comments_doc;
use core_format::doc::{Doc, concat, hardline};
use core_syntax::{SyntaxElement, SyntaxNode, SyntaxToken};
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    /// How many leading comments of a token the blocks before it have printed.
    static CONSUMED: RefCell<HashMap<u32, usize>> = RefCell::new(HashMap::new());
}

/// Starts a file: no comment has been printed yet.
pub(crate) fn reset() {
    CONSUMED.with(|c| c.borrow_mut().clear());
}

/// Records that `n` more leading comments of `tok` are printed.
pub(crate) fn consume(tok: &SyntaxToken, n: usize) {
    CONSUMED.with(|c| *c.borrow_mut().entry(offset(tok)).or_insert(0) += n);
}

/// Own-line comments one per line (a comment keeps the blank lines after it, at most one).
pub(crate) fn comments_block(cs: &[Comment]) -> Doc {
    let mut parts = Vec::new();
    for (i, c) in cs.iter().enumerate() {
        if i > 0 {
            parts.push(hardline());
            if cs[i - 1].breaks_after >= 2 {
                parts.push(hardline());
            }
        }
        parts.push(core_format::context::comment_doc(c));
    }
    concat(parts)
}

pub(crate) fn column(t: &SyntaxToken) -> usize {
    let mut col = 0;
    let mut p = t.prev_token();
    while let Some(x) = p {
        let s = x.text();
        if let Some(i) = s.rfind('\n') {
            col += s.len() - i - 1;
            break;
        }
        col += s.len();
        p = x.prev_token();
    }
    col
}

pub(crate) fn offset(t: &SyntaxToken) -> u32 {
    t.text_range().start().into()
}

/// The comments of `tok` that no block before it has printed.
pub(crate) fn remaining_comments(cx: &FormatCtx<'_>, tok: &SyntaxToken) -> Vec<Comment> {
    let n = CONSUMED.with(|c| c.borrow().get(&offset(tok)).copied().unwrap_or(0));
    let mut cs = comments::leading_comments(cx.grammar, tok);
    cs.drain(..n.min(cs.len()));
    cs
}

/// The next token after `node` that is not trivia.
pub(crate) fn next_token(cx: &FormatCtx<'_>, node: &SyntaxNode) -> Option<SyntaxToken> {
    let mut t = cx.last_token(node)?.next_token();
    while let Some(x) = t {
        if !cx.grammar.is_trivia(x.kind()) {
            return Some(x);
        }
        t = x.next_token();
    }
    None
}

/// Comments at the end of a block that stand at its indentation: the comments of the next token
/// that are indented at least as far as the block's statements.
pub(crate) fn end_comments(cx: &FormatCtx<'_>, last: &SyntaxNode, col: usize) -> Doc {
    let Some(next) = next_token(cx, last) else {
        return Doc::Nil;
    };
    cx.hold_leading(&next);
    let cs = remaining_comments(cx, &next);
    let take = cs
        .iter()
        .take_while(|c| column(c.token()) >= col && col > 0)
        .count();
    if take == 0 {
        return Doc::Nil;
    }
    CONSUMED.with(|c| *c.borrow_mut().entry(offset(&next)).or_insert(0) += take);
    let mut parts = Vec::new();
    for _ in 0..comments::blank_lines_before(cx.grammar, &next).min(1) {
        parts.push(hardline());
    }
    parts.push(hardline());
    parts.push(comments_block(&cs[..take]));
    concat(parts)
}

/// A token that starts a statement or clause, printed in front of its comments.
pub(crate) fn lead(cx: &FormatCtx<'_>, tok: &SyntaxToken) -> Doc {
    cx.hold_leading(tok);
    leading_comments_doc(&remaining_comments(cx, tok))
}

#[derive(PartialEq, Eq)]
pub(crate) enum Mark {
    Off,
    On,
}

pub(crate) fn consumed_of(tok: &SyntaxToken) -> usize {
    CONSUMED.with(|c| c.borrow().get(&offset(tok)).copied().unwrap_or(0))
}

/// `# fmt: off` and `# fmt: on` (also `# yapf: disable` and `enable`).
pub(crate) fn mark(cx: &FormatCtx<'_>, c: &Comment) -> Option<Mark> {
    let text = c.text();
    let body = cx.grammar.comment_body(&text).trim();
    match body {
        "fmt: off" | "fmt:off" | "yapf: disable" => Some(Mark::Off),
        "fmt: on" | "fmt:on" | "yapf: enable" => Some(Mark::On),
        _ => None,
    }
}

/// The `;` that ends a simple statement, if it has one: the sibling after the statement.
pub(crate) fn semicolon(cx: &FormatCtx<'_>, s: &SyntaxNode) -> Option<SyntaxToken> {
    let mut next = s.next_sibling_or_token();
    while let Some(e) = next {
        match e {
            SyntaxElement::Token(t) if t.kind() == k::SEMI => return Some(t),
            SyntaxElement::Token(t) if cx.grammar.is_trivia(t.kind()) => {
                next = t.next_sibling_or_token();
            }
            _ => return None,
        }
    }
    None
}

/// A simple statement with a trailing `# fmt: skip` (after its `;` when it has one).
pub(crate) fn is_fmt_skip(cx: &FormatCtx<'_>, s: &SyntaxNode) -> bool {
    !is_compound(s.kind())
        && semicolon(cx, s)
            .or_else(|| cx.last_token(s))
            .is_some_and(|l| {
                comments::trailing_comments(cx.grammar, &l).iter().any(|c| {
                    matches!(
                        cx.grammar.comment_body(&c.text()).trim(),
                        "fmt: skip" | "fmt:skip"
                    )
                })
            })
}

/// What formatting changes in the text that no layout decision can undo, done before the tree is
/// formatted (the safety check follows it): `#comment` becomes `# comment` (not `#!`, `#:`, `##`
/// or `#'`), a comment loses its trailing blanks, `class A():` loses its empty parentheses.
/// `None` when nothing changes.
pub(crate) fn rewrite(tree: &SyntaxNode, text: &str) -> Option<String> {
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    for e in tree.descendants_with_tokens() {
        match e {
            SyntaxElement::Token(t) if t.kind() == k::COMMENT => {
                let src = t.text();
                let trimmed = src.trim_end();
                let rest = &trimmed[1..];
                let fixed = match rest.chars().next() {
                    None => "#".to_string(),
                    Some(c) if " !:#'".contains(c) => trimmed.to_string(),
                    Some('\u{a0}') if !rest.trim_start().starts_with("type:") => {
                        format!("# {}", &rest['\u{a0}'.len_utf8()..])
                    }
                    Some(_) => format!("# {rest}"),
                };
                if fixed != src {
                    edits.push((
                        t.text_range().start().into(),
                        t.text_range().end().into(),
                        fixed,
                    ));
                }
            }
            SyntaxElement::Node(n) if n.kind() == k::STMT_CLASS_DEF => {
                let Some(args) = n.children().find(|c| c.kind() == k::ARGUMENTS) else {
                    continue;
                };
                let toks: Vec<SyntaxToken> = args
                    .descendants_with_tokens()
                    .filter_map(|e| e.into_token())
                    .collect();
                let significant: Vec<&SyntaxToken> = toks
                    .iter()
                    .filter(|t| !matches!(t.kind(), k::WHITESPACE | k::NEWLINE | k::CONTINUATION))
                    .collect();
                if let [open, close] = significant.as_slice()
                    && open.kind() == k::LPAR
                    && close.kind() == k::RPAR
                {
                    edits.push((
                        open.text_range().start().into(),
                        close.text_range().end().into(),
                        String::new(),
                    ));
                }
            }
            _ => {}
        }
    }
    if edits.is_empty() {
        return None;
    }
    edits.sort_by_key(|e| e.0);
    let mut out = String::new();
    let mut last = 0;
    for (start, end, fixed) in edits {
        out.push_str(&text[last..start]);
        out.push_str(&fixed);
        last = end;
    }
    out.push_str(&text[last..]);
    Some(out)
}
