//! Code rewrites as data: a formatter that also rewrites code (a short array syntax, a default
//! visibility) lists token rows, and the source is edited before it is laid out.
//!
//! A line is `PATTERN : ACTION`, the pattern as in [`crate::spacing`] and the action one of
//! `replace "x"` (the token becomes `x`), `replace through next "x"` (the token and the next one,
//! with the blanks between them, become `x`), `insert before "x"` and `insert after "x"`. The
//! first row that matches a token is the one applied. Lines starting with `#` are comments.

use crate::spacing::Pattern;
use core_syntax::{Grammar, SyntaxNode, SyntaxToken};

enum Action {
    Replace(String),
    ReplaceThroughNext(String),
    InsertBefore(String),
    InsertAfter(String),
}

pub struct RewriteRules(Vec<(Pattern, Action)>);

impl RewriteRules {
    pub fn parse(grammar: &Grammar, text: &str) -> RewriteRules {
        let rows = crate::rules::section(text, "rewrite")
            .map(|line| {
                let (pattern, action) = line
                    .split_once(" : ")
                    .unwrap_or_else(|| panic!("rewrite: `{line}`"));
                let (words, arg) = action
                    .split_once('"')
                    .unwrap_or_else(|| panic!("rewrite: `{line}`"));
                let arg = arg.trim_end_matches('"').to_string();
                let action = match words.trim() {
                    "replace" => Action::Replace(arg),
                    "replace through next" => Action::ReplaceThroughNext(arg),
                    "insert before" => Action::InsertBefore(arg),
                    "insert after" => Action::InsertAfter(arg),
                    other => panic!("rewrite: unknown action `{other}`"),
                };
                (Pattern::parse(grammar, pattern), action)
            })
            .collect();
        RewriteRules(rows)
    }

    /// The source with the rows applied, `None` when no token matches.
    pub fn apply(&self, grammar: &Grammar, tree: &SyntaxNode, text: &str) -> Option<String> {
        let mut edits: Vec<(usize, usize, &str)> = Vec::new();
        for t in tree
            .descendants_with_tokens()
            .filter_map(|e| e.into_token())
            .filter(|t| !grammar.is_trivia(t.kind()))
        {
            let Some((_, action)) = self.0.iter().find(|(p, _)| p.matches(&t)) else {
                continue;
            };
            let (start, end) = (
                usize::from(t.text_range().start()),
                usize::from(t.text_range().end()),
            );
            edits.push(match action {
                Action::Replace(x) => (start, end, x),
                Action::InsertBefore(x) => (start, start, x),
                Action::InsertAfter(x) => (end, end, x),
                Action::ReplaceThroughNext(x) => (start, next_end(grammar, &t)?, x),
            });
        }
        if edits.is_empty() {
            return None;
        }
        let mut out = String::with_capacity(text.len());
        let mut at = 0;
        for (from, to, with) in edits {
            out.push_str(&text[at..from]);
            out.push_str(with);
            at = to;
        }
        out.push_str(&text[at..]);
        Some(out)
    }
}

/// The end of the next token that is not whitespace or a comment.
fn next_end(grammar: &Grammar, t: &SyntaxToken) -> Option<usize> {
    let mut cur = t.next_token();
    while let Some(n) = cur {
        if !grammar.is_trivia(n.kind()) {
            return Some(usize::from(n.text_range().end()));
        }
        cur = n.next_token();
    }
    None
}
