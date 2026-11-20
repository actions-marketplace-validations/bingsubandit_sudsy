//! Optional parentheses around an operator expression: may they be left out because a bracket at
//! its start or end splits instead (ruff's `can_omit_optional_parentheses`)? The expression is
//! walked as an operand list: only one operator level may be in it, and only once, and the first
//! or last operand must end in a bracket pair that holds something. Which operator nodes count
//! and at what level is the language's ([`OmitHooks::level`]); the walk reads the node tags
//! `binary`, `unary`, `conditional`, `call`, `member-access`, `index`, `collection`, `display`
//! `rest` and `await`.
//!
//! concept: omit-optional-parentheses
//! what: Whether an expression's optional parentheses can be left out for a bracket that splits.
//! tools: ruff can_omit_optional_parentheses
//! reads: tag:binary, tag:unary, tag:conditional, tag:call, tag:index, tag:collection, tag:display

use crate::FormatCtx;
use crate::comments;
use crate::seq::kids;
use core_syntax::{SyntaxElement, SyntaxNode, SyntaxToken};

/// What only the language can judge.
pub trait OmitHooks {
    /// The level of the operators of `node` (higher binds tighter) when it takes part in the
    /// count; `None`: it is an operand (or, for a `unary` node, one that adds no level).
    fn level(&self, cx: &FormatCtx<'_>, node: &SyntaxNode) -> Option<u8>;
    /// Is `node` a pair of parentheses around an expression?
    fn is_paren(&self, node: &SyntaxNode) -> bool;
}

struct Walk<'a> {
    cx: &'a FormatCtx<'a>,
    hooks: &'a dyn OmitHooks,
    any_paren: bool,
    comments: bool,
    loosest: u8,
    count: usize,
    first: Option<SyntaxNode>,
    last: Option<SyntaxNode>,
}

fn has_comments(cx: &FormatCtx<'_>, t: &SyntaxToken) -> bool {
    !comments::leading_comments(cx.grammar, t).is_empty()
        || !comments::trailing_comments(cx.grammar, t).is_empty()
}

fn opens(e: Option<&SyntaxElement>) -> bool {
    matches!(e, Some(SyntaxElement::Token(t)) if matches!(t.text(), "(" | "[" | "{"))
}

impl Walk<'_> {
    fn update(&mut self, level: u8, n: usize) {
        if level < self.loosest {
            self.loosest = level;
            self.count = n;
        } else if level == self.loosest {
            self.count += n;
        }
    }

    fn tagged(&self, n: &SyntaxNode, tag: &str) -> bool {
        self.cx.grammar.has_tag(n.kind(), tag)
    }

    /// An operand: the first one starts the expression (a call or subscript starts with what it
    /// is applied to), the last one ends it.
    fn leaf(&mut self, n: &SyntaxNode) {
        if self.first.is_none() {
            let mut f = n.clone();
            while ["call", "member-access", "index"]
                .iter()
                .any(|t| self.tagged(&f, t))
            {
                match f.children().next() {
                    Some(c) => f = c,
                    None => break,
                }
            }
            self.first = Some(f);
        }
        self.last = Some(n.clone());
    }

    fn comment_between(&self, n: &SyntaxNode) -> bool {
        kids(self.cx, n).iter().any(|e| match e {
            SyntaxElement::Token(t) => has_comments(self.cx, t),
            SyntaxElement::Node(c) => self
                .cx
                .first_token(c)
                .is_some_and(|t| !comments::leading_comments(self.cx.grammar, &t).is_empty()),
        })
    }

    fn visit(&mut self, n: &SyntaxNode) {
        let level = self.hooks.level(self.cx, n);
        let binary = self.tagged(n, "binary");
        let unary = self.tagged(n, "unary");
        if (binary || unary && !self.tagged(n, "await")) && self.comment_between(n) {
            self.comments = true;
            return;
        }
        let ks = kids(self.cx, n);
        if let Some(level) = level {
            let count = if binary {
                // adjacent operator tokens (`is not`, `not in`) are one operator
                let mut runs = 0;
                let mut prev = false;
                for e in &ks {
                    let tok = e.as_token().is_some();
                    runs += usize::from(tok && !prev);
                    prev = tok;
                }
                runs.max(1)
            } else if self.tagged(n, "conditional") {
                2
            } else if unary {
                1
            } else {
                ks.iter().filter(|e| e.as_token().is_some()).count().max(1)
            };
            self.update(level, count);
        } else if !unary {
            if self.bracketed(n) || self.tagged(n, "call") || self.tagged(n, "index") {
                self.any_paren = true;
            }
            self.leaf(n);
            return;
        }
        // an operator in front (`not a`, `await a`) starts the whole expression
        if unary && self.first.is_none() {
            self.first = Some(n.clone());
        }
        for c in n.children() {
            self.visit(&c);
        }
    }

    /// Does the expression end in a bracket pair with something in it?
    fn bracketed(&self, n: &SyntaxNode) -> bool {
        let ks = kids(self.cx, n);
        if self.hooks.is_paren(n) {
            true
        } else if self.tagged(n, "display") || self.tagged(n, "collection") {
            ks.len() > 2 && opens(ks.first())
        } else if self.tagged(n, "call") {
            n.children()
                .find(|c| self.tagged(c, "arguments"))
                .is_some_and(|a| kids(self.cx, &a).len() > 2)
        } else if self.tagged(n, "rest") {
            n.children().next().is_some_and(|c| self.bracketed(&c))
        } else {
            false
        }
    }
}

/// May the optional parentheses around `n` be left out?
pub fn can_omit(cx: &FormatCtx<'_>, n: &SyntaxNode, hooks: &dyn OmitHooks) -> bool {
    let mut w = Walk {
        cx,
        hooks,
        any_paren: false,
        comments: false,
        loosest: u8::MAX,
        count: 0,
        first: None,
        last: None,
    };
    w.visit(n);
    if w.comments || !w.any_paren || w.count > 1 {
        return false;
    }
    if w.loosest == u8::MAX {
        return true;
    }
    let p = |x: &Option<SyntaxNode>| x.as_ref().is_some_and(|x| w.bracketed(x));
    p(&w.first) || p(&w.last)
}

/// Does the first line of `printed` (the content of `node`) fit after what stands in front of it
/// on its line: `prefix` columns, or when 0 the keywords before `node` (`return `, `if `), after
/// `indent` columns of indentation? Then the brackets split instead of the parentheses showing.
pub fn head_fits(
    cx: &FormatCtx<'_>,
    node: &SyntaxNode,
    prefix: usize,
    printed: &crate::doc::Doc,
    indent: usize,
    width: usize,
) -> bool {
    let lead = if prefix > 0 {
        prefix
    } else {
        let mut w = 0;
        let mut prev = node.prev_sibling_or_token();
        while let Some(p) = prev {
            match &p {
                SyntaxElement::Token(t) if !cx.grammar.is_trivia(t.kind()) => {
                    w += t.text().chars().count() + 1;
                }
                SyntaxElement::Token(_) => {}
                SyntaxElement::Node(_) => {
                    w = 0;
                    break;
                }
            }
            prev = p.prev_sibling_or_token();
        }
        w
    };
    indent + lead + printed.head_width() <= width
}
