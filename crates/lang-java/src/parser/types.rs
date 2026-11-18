//! Names, annotations, modifiers and types.
//!
//! concept: recursive-descent-parser

use super::P;
use crate::generated::kind as k;
use crate::lexer::T;
use core_format::deep;

impl P<'_> {
    /// `a.b.C` as nested `ScopedIdentifier`s (a name that is not an expression: packages,
    /// imports, annotations).
    pub(super) fn name(&mut self) {
        let cp = self.cp();
        if !self.at_name() {
            self.error("expected a name");
            return;
        }
        self.bump();
        while self.at(T::Dot) && self.nth(1).is_name() {
            self.wrap(cp, k::SCOPED_IDENTIFIER, |p| {
                p.bump();
                p.bump();
            });
        }
    }

    pub(super) fn annotations(&mut self) {
        while self.at(T::At) {
            self.annotation();
        }
    }

    /// `@Name` or `@Name(arguments)`.
    pub(super) fn annotation(&mut self) {
        let mut j = self.pos + 2;
        while self.at_idx(j) == T::Dot && self.at_idx(j + 1).is_name() {
            j += 2;
        }
        let has_args = self.at_idx(j) == T::LParen;
        let kind = if has_args {
            k::ANNOTATION
        } else {
            k::MARKER_ANNOTATION
        };
        self.node(kind, |p| {
            p.bump();
            p.name();
            if has_args {
                p.node(k::ANNOTATION_ARGUMENT_LIST, |p| {
                    p.bump();
                    p.list(T::RParen, |p| {
                        if p.at_name() && p.nth(1) == T::Eq {
                            p.node(k::ELEMENT_VALUE_PAIR, |p| {
                                p.bump();
                                p.bump();
                                p.element_value();
                            });
                        } else {
                            p.element_value();
                        }
                    });
                });
            }
        });
    }

    /// An annotation value: an expression, an array of values or an annotation.
    pub(super) fn element_value(&mut self) {
        match self.cur() {
            T::At => self.annotation(),
            T::LBrace => self.node(k::ELEMENT_VALUE_ARRAY_INITIALIZER, |p| {
                p.bump();
                p.list(T::RBrace, |p| p.element_value());
            }),
            _ => self.expr(),
        }
    }

    /// The items of a list up to its closing token (consumed), separated by commas; a trailing
    /// comma is fine.
    pub(super) fn list(&mut self, close: T, mut item: impl FnMut(&mut Self)) {
        while !self.at(close) && !self.at_eof() {
            let before = self.pos;
            item(self);
            if self.at(T::Comma) {
                self.bump();
            } else if !self.at(close) {
                if self.pos == before {
                    self.bump_error("unexpected token");
                } else {
                    // Something else follows an item: report and let the next item try.
                    self.error("expected `,`");
                    if matches!(self.cur(), T::Semi | T::RBrace | T::LBrace) {
                        break;
                    }
                }
            }
        }
        let name = match close {
            T::RParen => "`)`",
            T::RBrace => "`}`",
            T::RBracket => "`]`",
            _ => "a closing token",
        };
        self.expect(close, name);
    }

    /// Modifiers and annotations in any order, as one node of `kind` (none when there are none).
    pub(super) fn modifiers(&mut self, kind: core_syntax::SyntaxKind, member: bool) {
        if self.skip_modifiers(self.pos, member) == self.pos {
            return;
        }
        self.node(kind, |p| p.modifier_tokens(member));
    }

    /// The modifier keywords and annotations, as they are (a node of their own comes from the caller).
    pub(super) fn modifier_tokens(&mut self, member: bool) {
        loop {
            if self.at(T::At) {
                self.annotation();
                continue;
            }
            match self.modifier_len(self.pos, member) {
                0 => return,
                n => self.emit(k::KEYWORD, n),
            }
        }
    }

    /// A type, with leading annotations.
    pub(super) fn ty(&mut self) {
        deep(|| {
            if self.at(T::At) {
                self.node(k::ANNOTATED_TYPE, |p| {
                    p.annotations();
                    p.unannotated_type();
                });
            } else {
                self.unannotated_type();
            }
        })
    }

    /// A simple type with array dimensions.
    pub(super) fn unannotated_type(&mut self) {
        let cp = self.cp();
        self.simple_type();
        if self.dims_ahead() {
            self.wrap(cp, k::ARRAY_TYPE, |p| p.dimensions());
        }
    }

    fn dims_ahead(&self) -> bool {
        let j = self.skip_annotations(self.pos);
        self.at_idx(j) == T::LBracket && self.at_idx(j + 1) == T::RBracket
    }

    /// `[]` dimensions, each with its annotations.
    pub(super) fn dimensions(&mut self) {
        while self.dims_ahead() {
            self.node(k::DIMENSION, |p| {
                p.annotations();
                p.bump();
                p.bump();
            });
        }
    }

    /// A primitive type or a (scoped, generic) class type, without dimensions.
    pub(super) fn simple_type(&mut self) {
        let cp = self.cp();
        if self.cur().is_primitive() {
            self.bump();
            return;
        }
        if !self.at_name() {
            self.error("expected a type");
            return;
        }
        self.bump();
        loop {
            if self.at(T::Lt) {
                self.wrap(cp, k::GENERIC_TYPE, |p| p.type_arguments());
            } else if self.at(T::Dot) && (self.nth(1).is_name() || self.nth(1) == T::At) {
                self.wrap(cp, k::SCOPED_TYPE_NAME, |p| {
                    p.bump();
                    p.annotations();
                    if p.at_name() {
                        p.bump();
                    } else {
                        p.error("expected a name");
                    }
                });
            } else {
                return;
            }
        }
    }

    /// `<A, B>`, or the diamond `<>` as one token.
    pub(super) fn type_arguments(&mut self) {
        if self.nth(1) == T::Gt && self.adjacent(self.pos) {
            self.emit(k::OPERATOR, 2);
            return;
        }
        self.node(k::TYPE_ARGUMENTS, |p| {
            p.bump();
            while !p.at(T::Gt) && !p.at_eof() {
                let before = p.pos;
                p.type_argument();
                if !p.eat(T::Comma) {
                    if p.pos == before {
                        p.error("expected a type");
                    }
                    break;
                }
            }
            p.expect(T::Gt, "`>`");
        });
    }

    fn type_argument(&mut self) {
        let wildcard = self.skip_annotations(self.pos);
        if self.at_idx(wildcard) == T::Question {
            self.node(k::WILDCARD, |p| {
                p.annotations();
                p.bump();
                if matches!(p.cur(), T::Extends | T::Super) {
                    p.bump_kw();
                    p.ty();
                }
            });
        } else {
            self.ty();
        }
    }

    /// `<T extends A & B, U>`.
    pub(super) fn type_parameters(&mut self) {
        self.node(k::TYPE_PARAMETERS, |p| {
            p.bump();
            while !p.at(T::Gt) && !p.at_eof() {
                let before = p.pos;
                p.node(k::TYPE_PARAMETER, |p| {
                    p.annotations();
                    if p.at_name() {
                        p.bump();
                    } else {
                        p.error("expected a type parameter");
                    }
                    if p.at(T::Extends) {
                        p.node(k::TYPE_BOUND, |p| {
                            p.bump_kw();
                            p.ty();
                            while p.at(T::Amp) {
                                p.bump_as(k::OPERATOR);
                                p.ty();
                            }
                        });
                    }
                });
                if !p.eat(T::Comma) {
                    if p.pos == before {
                        p.error("expected a type parameter");
                    }
                    break;
                }
            }
            p.expect(T::Gt, "`>`");
        });
    }

    /// `, A, B` after a keyword: a list of types as `InterfaceTypeList`.
    pub(super) fn type_list(&mut self) {
        self.node(k::INTERFACE_TYPE_LIST, |p| {
            p.ty();
            while p.eat(T::Comma) {
                p.ty();
            }
        });
    }
}
