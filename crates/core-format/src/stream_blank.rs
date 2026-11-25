//! Blank lines of a line-preserving formatter that the source may lack or have too many of, decided
//! on the statements of the tree: a blank line after a guard clause (a modifier `if` or a block
//! whose only statement is a jump, with no `else`), around a statement that is a bare modifier
//! word (`private`), after the leading magic comments of a file and between definitions (not
//! between two definitions that each fit one line); none after the head of a body and before its
//! `end`. The kind sets and words are the plain facts of the language (`format-rows` of
//! `lang.yaml`).
//!
//! concept: stream-blank-lines
//! what: Blank lines added or removed around guard clauses, modifier words, magic comments, definitions and the edges of bodies.
//! tools: RuboCop Layout EmptyLineAfterGuardClause, EmptyLinesAroundAccessModifier, EmptyLineAfterMagicComment, EmptyLineBetweenDefs, EmptyLinesAroundClassBody
//! reads: path:stream_blank::BlankFacts

use crate::FormatCtx;
use crate::comments::Comment;
use crate::stream::Tk;
use crate::stream_indent::Lead;
use core_syntax::{Grammar, SyntaxKind, SyntaxNode};

/// The kind names and words of a language's blank-line facts.
#[derive(Clone, Copy)]
pub struct BlankFacts {
    /// The kind of a list of statements.
    pub statements: &'static [&'static str],
    /// Kinds of a conditional statement that can be a guard clause.
    pub guard: &'static [&'static str],
    /// Kinds of a statement that jumps (`return`, `break`).
    pub jump: &'static [&'static str],
    /// Names of calls that jump (`raise`).
    pub jump_words: &'static [&'static str],
    /// Kinds of the branch that makes a conditional more than a guard (`else`).
    pub branch: &'static [&'static str],
    /// Words that, alone as a statement, are modifiers of what follows (`private`).
    pub modifiers: &'static [&'static str],
    /// Keywords of a magic comment (`frozen_string_literal`).
    pub magic: &'static [&'static str],
    /// Starts of comments that belong to the code in front of them (`rubocop:`).
    pub directives: &'static [&'static str],
    /// Kinds whose body has no blank line at its start and end.
    pub body: &'static [&'static str],
    /// Kinds of definitions.
    pub defs: &'static [&'static str],
}

impl BlankFacts {
    pub const NONE: BlankFacts = BlankFacts {
        statements: &[],
        guard: &[],
        jump: &[],
        jump_words: &[],
        branch: &[],
        modifiers: &[],
        magic: &[],
        directives: &[],
        body: &[],
        defs: &[],
    };
}

#[derive(Default)]
pub struct Blanks {
    facts: Option<BlankFacts>,
    statements: Vec<SyntaxKind>,
    guard: Vec<SyntaxKind>,
    jump: Vec<SyntaxKind>,
    branch: Vec<SyntaxKind>,
    body: Vec<SyntaxKind>,
    defs: Vec<SyntaxKind>,
}

impl Blanks {
    pub fn new(g: &Grammar, facts: BlankFacts) -> Blanks {
        let kinds = |names: &[&str]| -> Vec<SyntaxKind> {
            (names.iter())
                .map(|n| {
                    g.kind_by_name(n)
                        .unwrap_or_else(|| panic!("format-rows: unknown kind `{n}`"))
                })
                .collect()
        };
        Blanks {
            facts: Some(facts),
            statements: kinds(facts.statements),
            guard: kinds(facts.guard),
            jump: kinds(facts.jump),
            branch: kinds(facts.branch),
            body: kinds(facts.body),
            defs: kinds(facts.defs),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.statements.is_empty()
    }

    /// Is the comment a directive of a tool (`# rubocop:disable`): it stays next to the code in
    /// front of it.
    pub fn is_directive(&self, c: &Comment) -> bool {
        let text = c.text();
        let text = text.trim_start_matches('#').trim_start();
        self.facts
            .is_some_and(|f| f.directives.iter().any(|d| text.starts_with(d)))
    }

    /// Is the comment a magic comment (`# frozen_string_literal: true`)?
    pub fn is_magic(&self, c: &Comment) -> bool {
        let text = c.text();
        let text = text
            .trim_start_matches('#')
            .trim_start_matches([' ', '-', '*']);
        let word = text
            .split([':', '=', ' '])
            .next()
            .unwrap_or_default()
            .replace('-', "_");
        self.facts.is_some_and(|f| f.magic.contains(&word.as_str()))
    }

    fn one_word(&self, cx: &FormatCtx<'_>, n: &SyntaxNode, words: &[&str]) -> bool {
        match (cx.first_token(n), cx.last_token(n)) {
            (Some(a), Some(b)) => a == b && words.contains(&a.text()),
            _ => false,
        }
    }

    /// A jump statement: of a jump kind, or a call of a jump word.
    fn is_jump(&self, cx: &FormatCtx<'_>, n: &SyntaxNode) -> bool {
        self.jump.contains(&n.kind())
            || cx
                .first_token(n)
                .is_some_and(|t| self.facts.is_some_and(|f| f.jump_words.contains(&t.text())))
    }

    /// A conditional whose only statement is a jump and that has no other branch.
    fn is_guard(&self, cx: &FormatCtx<'_>, tks: &[Tk], n: &SyntaxNode, single: bool) -> bool {
        if !self.guard.contains(&n.kind()) || n.children().any(|c| self.branch.contains(&c.kind()))
        {
            return false;
        }
        let mut lists = n.children().filter(|c| self.statements.contains(&c.kind()));
        match (lists.next(), lists.next()) {
            (Some(list), None) => {
                let mut stmts = list.children();
                matches!((stmts.next(), stmts.next()), (Some(s), None) if self.is_jump(cx, &s) && (!single || self.one_line(tks, &s)))
            }
            _ => false,
        }
    }

    fn one_line(&self, tks: &[Tk], n: &SyntaxNode) -> bool {
        let range = n.text_range();
        let a = tks.partition_point(|t| t.tok.text_range().start() < range.start());
        let b = tks.partition_point(|t| t.tok.text_range().start() < range.end());
        b > a && tks[a].first_line == tks[b - 1].last_line
    }

    /// Is token `i` the end of a body (its closing keyword)? No blank line stands in front of it.
    pub fn closes(&self, cx: &FormatCtx<'_>, tks: &[Tk], i: usize) -> bool {
        let tok = &tks[i].tok;
        std::iter::successors(tok.parent(), |n| n.parent())
            .take_while(|n| cx.last_token(n).is_some_and(|l| l == *tok))
            .any(|n| self.body.contains(&n.kind()))
    }

    /// `Some(true)`: a blank line has to stand in front of the line that starts with token `i`;
    /// `Some(false)`: none may; `None`: the source decides.
    pub fn wanted(&self, cx: &FormatCtx<'_>, tks: &[Tk], lead: Lead<'_>) -> Option<bool> {
        if self.is_empty() {
            return None;
        }
        // The outermost statement that starts with the token.
        let stmt = lead.outermost(&self.statements)?;
        let Some(prev) = stmt.prev_sibling() else {
            let list = stmt.parent()?;
            return list
                .parent()
                .is_some_and(|p| self.body.contains(&p.kind()))
                .then_some(false);
        };
        let words = self.facts.map_or(&[][..], |f| f.modifiers);
        let defs = |n: &SyntaxNode| self.defs.contains(&n.kind());
        let separate =
            defs(&prev) && defs(stmt) && !(self.one_line(tks, &prev) && self.one_line(tks, stmt));
        let required = self.one_word(cx, &prev, words)
            || self.one_word(cx, stmt, words)
            || separate
            || (self.is_guard(cx, tks, &prev, true)
                && !self.is_guard(cx, tks, stmt, false)
                && !self.is_jump(cx, stmt));
        required.then_some(true)
    }
}
