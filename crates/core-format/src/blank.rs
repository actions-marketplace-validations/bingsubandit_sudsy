//! `[blank]` rows: between which neighbouring statements or members a blank line is required
//! although the source has none. A row is `PREV / NEXT [: key=value ...]`; `PREV` and `NEXT` are
//! `*`, `kind=A|B` or `kind!=A|B`, and a row needs all its keys to hold (any row may decide).
//! Keys test the next node, or the previous one with the prefix `prev.`: `parent=A|B` (the next
//! node's parent), `same_kind=yes|no` (both of one kind), `has=A|B` and `lacks=A|B` (a child of
//! those kinds), `commented=yes|no` (a comment in front), `doc=yes|no` (a `/**` comment in
//! front), `comment_blank=yes|no` (a blank line between the comments in front or after them),
//! `annotation_args=yes|no` (an annotation with arguments among its modifiers) and `group=K`
//! (only when the second token of the two is of kind `K` for one of them).
//!
//! concept: blank-lines-between-members
//! what: Rows say between which neighbouring statements or members a blank line is required.
//! tools: ktlint blank-line-before-declaration, google-java-format member blank lines, Prettier blank-line preservation
//! reads: rules:blank

use crate::FormatCtx;
use crate::comments;
use crate::seq::{first, kids};
use core_syntax::{Grammar, SyntaxKind, SyntaxNode};

struct KindPat {
    kinds: Vec<SyntaxKind>,
    not: bool,
}

fn kinds_of(g: &Grammar, names: &str) -> Vec<SyntaxKind> {
    names
        .split('|')
        .filter(|n| !n.is_empty())
        .map(|n| {
            g.kind_by_name(n)
                .unwrap_or_else(|| panic!("blank: kind `{n}`"))
        })
        .collect()
}

impl KindPat {
    fn parse(g: &Grammar, text: &str) -> KindPat {
        let text = text.trim();
        let (not, names) = match text.strip_prefix("kind!=") {
            Some(n) => (true, n),
            None => (false, text.strip_prefix("kind=").unwrap_or("")),
        };
        KindPat {
            kinds: kinds_of(g, names),
            not,
        }
    }

    fn matches(&self, kind: SyntaxKind) -> bool {
        self.kinds.is_empty() || self.kinds.contains(&kind) != self.not
    }
}

enum Test {
    Parent(Vec<SyntaxKind>),
    SameKind,
    Has(Vec<SyntaxKind>),
    Commented,
    Doc,
    CommentBlank,
    AnnotationArgs,
    /// The second token of the node is of this kind.
    Second(SyntaxKind),
}

struct Cond {
    prev: bool,
    test: Test,
    want: bool,
}

struct Row {
    prev: KindPat,
    next: KindPat,
    conds: Vec<Cond>,
}

pub(crate) struct Blank(Vec<Row>);

impl Blank {
    pub(crate) fn parse<'a>(g: &Grammar, lines: impl Iterator<Item = &'a str>) -> Blank {
        Blank(
            lines
                .map(|l| {
                    let (pair, opts) = l.split_once(':').unwrap_or((l, ""));
                    let (prev, next) = pair
                        .split_once('/')
                        .unwrap_or_else(|| panic!("blank: `{l}`"));
                    let conds = opts
                        .split_whitespace()
                        .map(|w| {
                            let (key, value) = w.split_once('=').unwrap_or((w, ""));
                            let (prev, key) = match key.strip_prefix("prev.") {
                                Some(k) => (true, k),
                                None => (false, key),
                            };
                            let (test, want) = match key {
                                "parent" => (Test::Parent(kinds_of(g, value)), true),
                                "has" => (Test::Has(kinds_of(g, value)), true),
                                "lacks" => (Test::Has(kinds_of(g, value)), false),
                                "group" => (Test::Second(kinds_of(g, value)[0]), true),
                                _ => {
                                    let test = match key {
                                        "same_kind" => Test::SameKind,
                                        "commented" => Test::Commented,
                                        "doc" => Test::Doc,
                                        "comment_blank" => Test::CommentBlank,
                                        "annotation_args" => Test::AnnotationArgs,
                                        _ => panic!("blank: key `{key}`"),
                                    };
                                    (test, value == "yes")
                                }
                            };
                            Cond { prev, test, want }
                        })
                        .collect();
                    Row {
                        prev: KindPat::parse(g, prev),
                        next: KindPat::parse(g, next),
                        conds,
                    }
                })
                .collect(),
        )
    }

    pub(crate) fn between(&self, cx: &FormatCtx<'_>, prev: &SyntaxNode, next: &SyntaxNode) -> bool {
        self.0.iter().any(|r| {
            r.prev.matches(prev.kind())
                && r.next.matches(next.kind())
                && r.conds.iter().all(|c| {
                    let node = if c.prev { prev } else { next };
                    match &c.test {
                        // A `group` row holds when exactly one of the two is of the kind.
                        Test::Second(k) => second_is(cx, prev, *k) != second_is(cx, next, *k),
                        t => holds(cx, t, prev, next, node) == c.want,
                    }
                })
        })
    }
}

fn second_is(cx: &FormatCtx<'_>, node: &SyntaxNode, kind: SyntaxKind) -> bool {
    kids(cx, node)
        .get(1)
        .and_then(|e| first(cx, e))
        .is_some_and(|t| t.kind() == kind)
}

fn leading(cx: &FormatCtx<'_>, node: &SyntaxNode) -> Vec<comments::Comment> {
    cx.first_token(node)
        .map(|t| comments::leading_comments(cx.grammar, &t))
        .unwrap_or_default()
}

fn holds(
    cx: &FormatCtx<'_>,
    test: &Test,
    prev: &SyntaxNode,
    next: &SyntaxNode,
    node: &SyntaxNode,
) -> bool {
    match test {
        Test::Parent(kinds) => next.parent().is_some_and(|p| kinds.contains(&p.kind())),
        Test::SameKind => prev.kind() == next.kind(),
        Test::Has(kinds) => node.children().any(|c| kinds.contains(&c.kind())),
        Test::Commented => !leading(cx, node).is_empty(),
        Test::Doc => crate::shapes::has_doc_comment(cx, node),
        Test::CommentBlank => leading(cx, node).iter().any(|c| c.breaks_after >= 2),
        Test::AnnotationArgs => crate::shapes::annotation_args(cx, node),
        Test::Second(_) => false,
    }
}
