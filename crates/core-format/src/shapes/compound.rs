//! Compound statements: a head, a body and the continuation clauses that follow it (`else`, the
//! `while` of a `do`). A body that is a block (`#block`) or an empty statement
//! (`#empty-statement`) stays on the head's line; any other statement goes on the next line,
//! indented, when it does not fit. A continuation keyword joins the `}` of a block body and
//! stands on a line of its own after any other body.
//!
//! concept: compound-statement
//! what: A head, a body and the continuation clauses (else, while of do) of a compound statement.
//! tools: Prettier if, while and do statements, google-java-format control statements
//! reads: tag:empty-statement

use crate::FormatCtx;
use crate::comments::{self, CommentKind};
use crate::doc::{self, Doc};
use crate::seq::{first, last};
use core_syntax::{SyntaxElement, SyntaxToken};

/// Where a body that is not a block goes (a language option such as `short_ifs`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Join {
    /// On the next line when the enclosing group breaks (the head's group).
    Hang,
    /// On the head's line when the whole fits, else on the next line.
    Fits,
    /// Always on the next line.
    Never,
}

/// The body of a compound statement after its head. `inline`: a body that is not a block still
/// follows on the line (`else if`); `join`: where any other body goes.
pub fn body(
    cx: &FormatCtx<'_>,
    body: &SyntaxElement,
    printed: Doc,
    inline: bool,
    join: Join,
) -> Doc {
    let (g, kind) = (cx.grammar, body.kind());
    // (`while (x);`; with a comment before the `;` it is a body like any other)
    let bare = first(cx, body).is_none_or(|t| comments::leading_comments(g, &t).is_empty());
    if g.has_tag(kind, "empty-statement") && bare {
        printed
    } else if inline || g.has_tag(kind, "block") {
        doc::concat(vec![gap_before(cx, body), printed])
    } else {
        moved(printed, join)
    }
}

/// A body on the line after its head, or joined to it when it fits.
fn moved(printed: Doc, join: Join) -> Doc {
    match join {
        Join::Hang => doc::indent(doc::concat(vec![doc::line(), printed])),
        Join::Fits => doc::group(doc::indent(doc::concat(vec![doc::line(), printed]))),
        Join::Never => doc::indent(doc::concat(vec![doc::hardline(), printed])),
    }
}

/// A blank before a body, or a line break when a line comment ends the line of the keyword in
/// front of it (`else // c`).
fn gap_before(cx: &FormatCtx<'_>, body: &SyntaxElement) -> Doc {
    let prev = first(cx, body)
        .and_then(|t| t.prev_token())
        .and_then(|mut t| {
            while cx.grammar.is_trivia(t.kind()) {
                t = t.prev_token()?;
            }
            Some(t)
        });
    let ends_line = prev.is_some_and(|t| {
        !matches!(t.text(), ")" | ";" | "]" | "}")
            && comments::trailing_comments(cx.grammar, &t)
                .iter()
                .any(|c| c.kind != CommentKind::Block)
    });
    if ends_line {
        doc::hardline()
    } else {
        doc::lit(" ")
    }
}

/// What stands between `prev` (the body before a continuation keyword `kw`) and `kw`: a blank
/// after a block, a line break after anything else or after a line comment, and the blank line
/// the source has before a comment in front of `kw`.
pub fn continuation(cx: &FormatCtx<'_>, prev: &SyntaxElement, kw: &SyntaxToken) -> Doc {
    let g = cx.grammar;
    let line_comment = last(cx, prev).is_some_and(|t| {
        comments::trailing_comments(g, &t)
            .iter()
            .any(|c| c.kind == CommentKind::Line)
    });
    let mut parts = vec![if g.has_tag(prev.kind(), "block") && !line_comment {
        doc::lit(" ")
    } else {
        doc::hardline()
    }];
    if !comments::leading_comments(g, kw).is_empty() && blank_before_first_comment(cx, kw) {
        parts.extend([doc::hardline(), doc::hardline()]);
    }
    doc::concat(parts)
}

/// Do two line breaks come before the first comment in front of `tok`?
fn blank_before_first_comment(cx: &FormatCtx<'_>, tok: &SyntaxToken) -> bool {
    let mut run = Vec::new();
    let mut cur = tok.prev_token();
    while let Some(t) = cur.filter(|t| cx.grammar.is_trivia(t.kind())) {
        cur = t.prev_token();
        run.push(t);
    }
    run.iter()
        .rev()
        .take_while(|t| !cx.grammar.has_tag(t.kind(), "comment"))
        .map(|t| comments::count_breaks(t.text()))
        .sum::<usize>()
        > 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::printer::{PrintOptions, print};

    fn render(join: Join) -> String {
        let mut d = doc::concat(vec![doc::lit("if (a)"), moved(doc::lit("b();"), join)]);
        print(&mut d, &PrintOptions::default())
    }

    #[test]
    fn a_body_joins_the_head_only_when_the_policy_lets_it_fit() {
        assert_eq!(render(Join::Fits), "if (a) b();");
        assert_eq!(render(Join::Never), "if (a)\n  b();");
    }
}
