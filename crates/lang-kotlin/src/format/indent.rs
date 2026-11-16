//! The indentation of a line, in levels. A construct that is open at the line (a bracket or brace
//! opened before it, an expression or declaration begun before it and continued on it) adds one
//! level; constructs that begin on the same line add one level together. A line that starts with
//! the closer of a construct belongs to the level of the line the construct opened on.

use super::src::Source;
use crate::generated::kind as k;
use core_syntax::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

pub(super) struct Indent<'a> {
    pub src: &'a Source,
    /// The output line of each token.
    pub line_of: &'a [u32],
}

pub(super) fn is_trivia(kind: SyntaxKind) -> bool {
    kind == k::WHITESPACE || kind == k::NEWLINE || kind == k::COMMENT
}

fn opens(kind: SyntaxKind) -> bool {
    matches!(kind, k::LPAR | k::LBRACE | k::LBRACKET | k::LT)
}

pub(super) fn closes(kind: SyntaxKind) -> bool {
    matches!(kind, k::RPAR | k::RBRACE | k::RBRACKET | k::GT)
}

/// The parts of a class header after its name that may continue on later lines.
fn is_header_part(e: &SyntaxElement) -> bool {
    matches!(
        e.kind(),
        k::PRIMARY_CONSTRUCTOR | k::COLON | k::SUPER_TYPE_LIST | k::TYPE_CONSTRAINT_LIST
    ) || e.as_token().is_some_and(|t| t.text() == "where")
}

fn same(a: &SyntaxElement, b: &SyntaxElement) -> bool {
    a.text_range() == b.text_range() && a.kind() == b.kind()
}

fn first_sig_child(node: &SyntaxNode) -> Option<SyntaxElement> {
    node.children_with_tokens().find(|c| !is_trivia(c.kind()))
}

impl Indent<'_> {
    fn first_idx(&self, node: &SyntaxNode) -> usize {
        self.src.index_at(node.text_range().start().into())
    }

    fn idx_of(&self, t: &SyntaxToken) -> usize {
        self.src.index_at(t.text_range().start().into())
    }

    /// The token the indent a node adds to `child` hangs from.
    fn anchor(&self, a: &SyntaxNode, child: &SyntaxElement) -> Option<usize> {
        match a.kind() {
            k::CLASS_BODY
            | k::BLOCK
            | k::FUNCTION_LITERAL
            | k::VALUE_ARGUMENT_LIST
            | k::VALUE_PARAMETER_LIST
            | k::TYPE_ARGUMENT_LIST
            | k::TYPE_PARAMETER_LIST
            | k::PARENTHESIZED
            | k::INDICES
            | k::COLLECTION_LITERAL_EXPRESSION
            | k::CONTEXT_PARAMETER_LIST => {
                let open = first_sig_child(a)?
                    .into_token()
                    .filter(|t| opens(t.kind()))?;
                if a.kind() == k::CLASS_BODY {
                    // The members hang from the line the declaration starts on, not from the `{`.
                    let delimiter = child
                        .as_token()
                        .is_some_and(|t| closes(t.kind()) || t.text_range() == open.text_range());
                    return (!delimiter).then(|| self.decl_idx(&a.parent()?)).flatten();
                }
                let delimiter = child.as_token().is_some_and(|t| {
                    closes(t.kind()) || same(&t.clone().into(), &open.clone().into())
                });
                (!delimiter).then(|| self.idx_of(&open))
            }
            k::IF
            | k::WHILE
            | k::DO_WHILE
            | k::FOR
            | k::WHEN
            | k::CATCH
            | k::DESTRUCTURING_DECLARATION => self.pairs(a, child),
            k::BINARY_EXPRESSION
            | k::BINARY_WITH_TYPE
            | k::IS_EXPRESSION
            | k::DOT_QUALIFIED_EXPRESSION
            | k::SAFE_ACCESS_EXPRESSION => {
                let first = self.first_idx(a);
                let c = self.first_idx_of(child);
                // The continuation part is indented when it starts on a later line than the
                // first operand ends, or when an operand of a chain before it did so.
                let head = first_sig_child(a)?;
                let head_last = self.last_idx(&head)?;
                let broken = c != first && self.line_of[c] > self.line_of[head_last];
                (c != first && (broken || self.chain_hangs(&head))).then_some(first)
            }
            k::PROPERTY_DELEGATE => self.hang(self.first_idx(a), child),
            k::PROPERTY
            | k::FUN
            | k::TYPEALIAS
            | k::PROPERTY_ACCESSOR
            | k::VALUE_PARAMETER
            | k::VALUE_ARGUMENT => {
                if matches!(child.kind(), k::PROPERTY_ACCESSOR | k::BACKING_FIELD) {
                    return self.hang(self.first_idx(a), child);
                }
                let eq = a
                    .children_with_tokens()
                    .find_map(|c| c.into_token().filter(|t| t.kind() == k::EQ))?;
                if child.text_range().start() < eq.text_range().end() {
                    return None;
                }
                self.hang(self.idx_of(&eq), child)
            }
            k::CLASS | k::OBJECT_DECLARATION => {
                // The line of the keyword: modifiers and annotations may come before it.
                let keyword = a
                    .children_with_tokens()
                    .find_map(|c| c.into_token().filter(|t| !is_trivia(t.kind())))?;
                let first = self.idx_of(&keyword);
                let later = self.line_of[self.first_idx_of(child)] > self.line_of[first];
                (is_header_part(child) && later).then_some(first)
            }
            k::SUPER_TYPE_LIST | k::TYPE_CONSTRAINT_LIST => {
                let head = first_sig_child(a)?;
                let head_last = self.last_idx(&head)?;
                let c = self.first_idx_of(child);
                (c != self.first_idx(a) && self.line_of[c] > self.line_of[head_last])
                    .then(|| self.first_idx(a))
            }
            k::WHEN_ENTRY => {
                let arrow = a
                    .children_with_tokens()
                    .find_map(|c| c.into_token().filter(|t| t.kind() == k::ARROW))?;
                if child.text_range().start() < arrow.text_range().end() {
                    return None;
                }
                self.hang(self.idx_of(&arrow), child)
            }
            _ => None,
        }
        .or_else(|| self.body_anchor(a, child))
    }

    /// A branch or loop body on a line of its own hangs from the token before it.
    fn body_anchor(&self, a: &SyntaxNode, child: &SyntaxElement) -> Option<usize> {
        if !matches!(a.kind(), k::IF | k::FOR | k::WHILE | k::DO_WHILE)
            || !matches!(child.kind(), k::THEN | k::ELSE | k::BODY)
        {
            return None;
        }
        // `else if` continues the chain at the same level.
        if child.kind() == k::ELSE
            && child
                .as_node()
                .and_then(first_sig_child)
                .is_some_and(|c| c.kind() == k::IF)
        {
            return None;
        }
        let first = self.src.index_at(child.text_range().start().into());
        self.hang(first.checked_sub(1)?, child)
    }

    /// `anchor` holds the indent of `child` when `child` starts on a line after the token before it.
    fn hang(&self, anchor: usize, child: &SyntaxElement) -> Option<usize> {
        let c = self.first_idx_of(child);
        (c > 0 && self.line_of[c] > self.line_of[c - 1]).then_some(anchor)
    }

    fn last_idx(&self, e: &SyntaxElement) -> Option<usize> {
        self.src
            .index_at(e.text_range().end().into())
            .checked_sub(1)
    }

    /// Does a part of the chain `e` (a binary or qualified expression) start a line?
    fn chain_hangs(&self, e: &SyntaxElement) -> bool {
        let Some(n) = e.as_node() else { return false };
        if !matches!(
            n.kind(),
            k::BINARY_EXPRESSION
                | k::DOT_QUALIFIED_EXPRESSION
                | k::SAFE_ACCESS_EXPRESSION
                | k::BINARY_WITH_TYPE
                | k::IS_EXPRESSION
        ) {
            return false;
        }
        let mut sig = n.children_with_tokens().filter(|c| !is_trivia(c.kind()));
        let Some(head) = sig.next() else { return false };
        let Some(head_last) = self.last_idx(&head) else {
            return false;
        };
        let second = sig.next().map(|c| self.first_idx_of(&c));
        second.is_some_and(|c| self.line_of[c] > self.line_of[head_last]) || self.chain_hangs(&head)
    }

    /// The keyword of a class or object declaration (modifiers and annotations come before it).
    fn decl_idx(&self, decl: &SyntaxNode) -> Option<usize> {
        let keyword = decl
            .children_with_tokens()
            .find_map(|c| c.into_token().filter(|t| !is_trivia(t.kind())))?;
        Some(self.idx_of(&keyword))
    }

    fn first_idx_of(&self, e: &SyntaxElement) -> usize {
        self.src.index_at(e.text_range().start().into())
    }

    /// The opener of the pair of parentheses or braces of `a` that `child` lies in.
    fn pairs(&self, a: &SyntaxNode, child: &SyntaxElement) -> Option<usize> {
        let mut open: Option<SyntaxToken> = None;
        for c in a.children_with_tokens() {
            if same(&c, child) {
                let delimiter = child
                    .as_token()
                    .is_some_and(|t| opens(t.kind()) || closes(t.kind()));
                return if delimiter {
                    None
                } else {
                    open.map(|t| self.idx_of(&t))
                };
            }
            if let Some(t) = c.as_token() {
                if opens(t.kind()) {
                    open = Some(t.clone());
                } else if closes(t.kind()) {
                    open = None;
                }
            }
        }
        None
    }

    /// The line the construct `closer` ends was opened on.
    fn opened_on(&self, closer: &SyntaxToken) -> Option<u32> {
        let parent = closer.parent()?;
        let mut stack: Vec<SyntaxToken> = Vec::new();
        for c in parent.children_with_tokens() {
            let Some(t) = c.as_token() else { continue };
            if t.text_range() == closer.text_range() {
                return stack.last().map(|o| self.line_of[self.idx_of(o)]);
            }
            if opens(t.kind()) {
                stack.push(t.clone());
            } else if closes(t.kind()) {
                stack.pop();
            }
        }
        None
    }

    /// The indentation of the line token `i` starts: the indentation of the line the innermost
    /// open construct hangs from, plus one level. A closer takes the indentation of the line its
    /// construct opened on. `line_ws` holds the indentation of the lines already printed, and
    /// `written` the indentation the source has before the token.
    pub fn indent_of(&self, i: usize, line_ws: &[String], unit: &str, written: &str) -> String {
        let t = &self.src.toks[i];
        let line = self.line_of[i];
        let mut inner: Option<u32> = None;
        let mut child: SyntaxElement = t.clone().into();
        let mut parent = t.parent();
        while let Some(a) = parent {
            if a.kind() == k::PRIMARY_CONSTRUCTOR && self.first_idx(&a) == i {
                // A constructor on a line of its own stays where the author put it.
                return written.to_string();
            }
            if let Some(at) = self.anchor(&a, &child) {
                let l = self.line_of[at];
                if l < line && inner.is_none_or(|m| l > m) {
                    inner = Some(l);
                }
            }
            child = a.clone().into();
            parent = a.parent();
        }
        if closes(t.kind())
            && let Some(l) = self.opened_on(t)
        {
            let l = match t.parent() {
                Some(p) if p.kind() == k::CLASS_BODY => p
                    .parent()
                    .and_then(|d| self.decl_idx(&d))
                    .map_or(l, |d| self.line_of[d]),
                _ => l,
            };
            return line_ws[l as usize].clone();
        }
        inner.map_or_else(String::new, |l| format!("{}{unit}", line_ws[l as usize]))
    }
}
