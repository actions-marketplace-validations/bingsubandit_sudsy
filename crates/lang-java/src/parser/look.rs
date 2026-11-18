//! Lookahead over the tokens, with no tree built: where a type, an annotation or a run of
//! modifiers ends, and what a `(` opens. Java needs it where a declaration, a cast, a lambda and
//! an expression start alike.
//!
//! concept: new parser-lookahead
//! what: Token lookahead without building a tree, to decide what a construct is.

use super::P;
use crate::lexer::T;

/// Types nested deeper than this are not read ahead (the parser reads them with `core_format::deep`).
const MAX_TYPE_DEPTH: u8 = 64;

#[derive(PartialEq, Eq, Clone, Copy)]
pub(super) enum Paren {
    Lambda,
    Cast,
    Plain,
}

impl P<'_> {
    /// The end of the annotation at `i` (a `@`), or `i` when there is none.
    pub(super) fn skip_annotation(&self, i: usize) -> usize {
        if self.at_idx(i) != T::At || !self.at_idx(i + 1).is_name() {
            return i;
        }
        let mut j = i + 2;
        while self.at_idx(j) == T::Dot && self.at_idx(j + 1).is_name() {
            j += 2;
        }
        if self.at_idx(j) == T::LParen {
            match self.close.get(j) {
                Some(&c) if c != u32::MAX => j = c as usize + 1,
                _ => {}
            }
        }
        j
    }

    pub(super) fn skip_annotations(&self, mut i: usize) -> usize {
        loop {
            let j = self.skip_annotation(i);
            if j == i {
                return i;
            }
            i = j;
        }
    }

    /// The end of the type that starts at `i`, with its `[]` suffixes when `dims`.
    pub(super) fn skip_type(&self, i: usize, dims: bool) -> Option<usize> {
        self.skip_type_at(i, dims, 0)
    }

    fn skip_type_at(&self, i: usize, dims: bool, depth: u8) -> Option<usize> {
        let mut i = self.skip_annotations(i);
        let t = self.at_idx(i);
        if t.is_primitive() {
            i += 1;
        } else if t.is_name() {
            i += 1;
            loop {
                if self.at_idx(i) == T::Lt {
                    i = self.skip_type_args(i, depth + 1)?;
                }
                if self.at_idx(i) == T::Dot
                    && (self.at_idx(i + 1).is_name() || self.at_idx(i + 1) == T::At)
                {
                    let j = self.skip_annotations(i + 1);
                    if !self.at_idx(j).is_name() {
                        return None;
                    }
                    i = j + 1;
                } else {
                    break;
                }
            }
        } else {
            return None;
        }
        if dims {
            loop {
                let j = self.skip_annotations(i);
                if self.at_idx(j) == T::LBracket && self.at_idx(j + 1) == T::RBracket {
                    i = j + 2;
                } else {
                    break;
                }
            }
        }
        Some(i)
    }

    /// The end of the type arguments at `i` (a `<`).
    pub(super) fn skip_type_args(&self, i: usize, depth: u8) -> Option<usize> {
        if depth > MAX_TYPE_DEPTH {
            return None;
        }
        let mut i = i + 1;
        if self.at_idx(i) == T::Gt {
            return Some(i + 1);
        }
        loop {
            i = self.skip_annotations(i);
            if self.at_idx(i) == T::Question {
                i += 1;
                if matches!(self.at_idx(i), T::Extends | T::Super) {
                    i = self.skip_type_at(i + 1, true, depth)?;
                }
            } else {
                i = self.skip_type_at(i, true, depth)?;
            }
            match self.at_idx(i) {
                T::Comma => i += 1,
                T::Gt => return Some(i + 1),
                _ => return None,
            }
        }
    }

    /// The number of tokens of the modifier at `i` (none: 0). Annotations are not counted.
    pub(super) fn modifier_len(&self, i: usize, member: bool) -> usize {
        match self.at_idx(i) {
            T::Public
            | T::Protected
            | T::Private
            | T::Abstract
            | T::Static
            | T::Final
            | T::Strictfp
            | T::Native
            | T::Transient
            | T::Volatile => 1,
            T::Synchronized | T::Default if member => 1,
            T::Sealed
                if matches!(
                    self.at_idx(i + 1),
                    T::Class
                        | T::Interface
                        | T::Abstract
                        | T::Public
                        | T::Protected
                        | T::Private
                        | T::Static
                        | T::Final
                        | T::Strictfp
                        | T::At
                        | T::Non
                ) =>
            {
                1
            }
            T::Non
                if self.at_idx(i + 1) == T::Minus
                    && self.at_idx(i + 2) == T::Sealed
                    && self.adjacent(i)
                    && self.adjacent(i + 1) =>
            {
                3
            }
            _ => 0,
        }
    }

    /// The end of the modifiers and annotations that start at `i`.
    pub(super) fn skip_modifiers(&self, mut i: usize, member: bool) -> usize {
        loop {
            let j = self.skip_annotation(i);
            if j != i {
                i = j;
                continue;
            }
            match self.modifier_len(i, member) {
                0 => return i,
                n => i += n,
            }
        }
    }

    /// What the `(` at the current token opens.
    pub(super) fn paren_kind(&self) -> Paren {
        let open = self.pos;
        let close = match self.close.get(open) {
            Some(&c) if c != u32::MAX => c as usize,
            _ => return Paren::Plain,
        };
        if self.at_idx(close + 1) == T::Arrow && !self.no_lambda {
            return Paren::Lambda;
        }
        let Some(end) = self.skip_type(open + 1, true) else {
            return Paren::Plain;
        };
        let first = self.at_idx(open + 1);
        // A type, or types joined by `&`, and nothing else between the parentheses.
        let mut j = end;
        while self.at_idx(j) == T::Amp {
            match self.skip_type(j + 1, true) {
                Some(e) => j = e,
                None => return Paren::Plain,
            }
        }
        if j != close {
            return Paren::Plain;
        }
        if first.is_primitive() && end == open + 2 {
            return Paren::Cast;
        }
        let next = self.at_idx(close + 1);
        let starts_operand = next.is_name()
            || next.is_primitive()
            || matches!(
                next,
                T::NumberLit
                    | T::StringLit
                    | T::CharLit
                    | T::True
                    | T::False
                    | T::Null
                    | T::LParen
                    | T::Not
                    | T::Tilde
                    | T::This
                    | T::Super
                    | T::New
                    | T::Void
                    | T::Switch
            );
        // `(a) + b` is a sum, `(int) +b` is a cast.
        if starts_operand || (first.is_primitive() && matches!(next, T::Plus | T::Minus)) {
            Paren::Cast
        } else {
            Paren::Plain
        }
    }
}
