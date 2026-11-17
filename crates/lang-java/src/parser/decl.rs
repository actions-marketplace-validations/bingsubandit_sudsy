//! The file, type declarations and their members.

use super::P;
use crate::generated::kind as k;
use crate::lexer::T;
use core_format::deep;
use core_syntax::SyntaxKind;

/// What kind of body a member is read in.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Body {
    Class,
    Interface,
    Record,
    Annotation,
}

impl P<'_> {
    pub(super) fn compilation_unit(&mut self) {
        self.node(k::COMPILATION_UNIT, |p| {
            let mut unnamed = false;
            let j = p.skip_annotations(p.pos);
            if p.at_idx(j) == T::Package {
                p.node(k::PACKAGE_DECLARATION, |p| {
                    p.annotations();
                    p.bump_kw();
                    p.name();
                    p.semi();
                });
            }
            while !p.at_eof() {
                let before = p.pos;
                p.top_level_item(&mut unnamed);
                if p.pos == before {
                    p.bump_error("unexpected token");
                }
            }
            if unnamed {
                p.finish();
                p.finish();
            }
        });
    }

    fn top_level_item(&mut self, unnamed: &mut bool) {
        if self.at(T::Import) {
            self.import_declaration();
            return;
        }
        if self.at(T::Semi) {
            self.node(k::TOP_LEVEL_DECLARATION, |p| p.bump());
            return;
        }
        let j = self.skip_annotations(self.pos);
        let module = match self.at_idx(j) {
            T::Open => self.at_idx(j + 1) == T::Module,
            T::Module => self.at_idx(j + 1).is_name(),
            _ => false,
        };
        if module {
            self.node(k::UNNAMED_COMPILATION_UNIT, |p| p.module_declaration());
        } else if self.type_declaration_ahead() {
            self.node(k::TOP_LEVEL_DECLARATION, |p| {
                p.type_declaration();
            });
        } else if self.at(T::RBrace) {
            self.bump_error("unexpected `}`");
        } else {
            // A method or field outside a class: a compact source file.
            if !*unnamed {
                *unnamed = true;
                self.start(k::UNNAMED_COMPILATION_UNIT);
                self.start(k::UNIT_BEFORE_FIRST_METHOD);
            }
            self.member(Body::Class);
        }
    }

    fn import_declaration(&mut self) {
        self.node(k::IMPORT_DECLARATION, |p| {
            if p.nth(1) == T::Module && p.nth(2).is_name() {
                p.bump_kw();
                p.bump_kw();
                p.name();
                p.semi();
                return;
            }
            p.node(k::TYPE_OR_STATIC_IMPORT_DECLARATION, |p| {
                p.bump_kw();
                if p.at(T::Static) {
                    p.bump_kw();
                }
                p.name();
                if p.at(T::Dot) && p.nth(1) == T::Star {
                    p.bump();
                    p.bump_as(k::OPERATOR);
                }
                p.semi();
            });
        });
    }

    fn module_declaration(&mut self) {
        self.node(k::MODULE_DECLARATION, |p| {
            p.annotations();
            if p.at(T::Open) {
                p.bump_kw();
            }
            p.bump_kw();
            p.name();
            p.node(k::MODULE_BODY, |p| {
                p.expect(T::LBrace, "`{`");
                while !p.at(T::RBrace) && !p.at_eof() {
                    p.node(k::MODULE_DIRECTIVE, |p| {
                        p.bump_kw();
                        while !p.at(T::Semi) && !p.at(T::RBrace) && !p.at_eof() {
                            match p.cur() {
                                T::Transitive | T::Static | T::To | T::With => p.bump_kw(),
                                T::Comma => p.bump(),
                                t if t.is_name() => p.name(),
                                _ => p.bump_error("unexpected token"),
                            }
                        }
                        p.semi();
                    });
                }
                p.expect(T::RBrace, "`}`");
            });
        });
    }

    // Type declarations

    /// Whether modifiers and a type keyword (`class`, `record Name(`, ...) start here.
    fn type_declaration_ahead(&self) -> bool {
        self.declared_type_kind().is_some()
    }

    /// The wrapper node of the type declaration that starts here, after its modifiers.
    fn declared_type_kind(&self) -> Option<SyntaxKind> {
        let j = self.skip_modifiers(self.pos, true);
        Some(match self.at_idx(j) {
            T::Class => k::CLASS_DECLARATION,
            T::Interface => k::INTERFACE_DECLARATION,
            T::Enum => k::ENUM_DECLARATION,
            T::AtInterface => k::ANNOTATION_TYPE_DECLARATION,
            T::Record
                if self.at_idx(j + 1).is_name()
                    && matches!(self.at_idx(j + 2), T::LParen | T::Lt) =>
            {
                k::RECORD_DECLARATION
            }
            _ => return None,
        })
    }

    /// A type declaration with its modifiers; false when none starts here.
    fn type_declaration(&mut self) -> bool {
        let Some(kind) = self.declared_type_kind() else {
            return false;
        };
        self.node(kind, |p| {
            p.modifiers(k::MODIFIERS, true);
            p.type_declaration_core();
        });
        true
    }

    /// From the type keyword on: `class A<T> extends B implements C { ... }`.
    pub(super) fn type_declaration_core(&mut self) {
        let (core, body) = match self.cur() {
            T::Class => (k::CLASS_DECLARATION_CORE, k::CLASS_BODY),
            T::Interface => (k::INTERFACE_DECLARATION_CORE, k::INTERFACE_BODY),
            T::Enum => (k::ENUM_DECLARATION_CORE, k::ENUM_BODY),
            T::AtInterface => (k::ANNOTATION_TYPE_DECLARATION_CORE, k::ANNOTATION_TYPE_BODY),
            _ => (k::RECORD_DECLARATION_CORE, k::RECORD_BODY),
        };
        let kw = self.cur();
        self.node(core, |p| {
            p.bump_kw();
            if p.at_name() {
                p.bump();
            } else {
                p.error("expected a name");
            }
            if p.at(T::Lt) {
                p.type_parameters();
            }
            if kw == T::Record {
                p.record_header();
            }
            if p.at(T::Extends) {
                let kind = if kw == T::Interface {
                    k::EXTENDS_INTERFACES
                } else {
                    k::SUPERCLASS
                };
                p.node(kind, |p| {
                    p.bump_kw();
                    if kw == T::Interface {
                        p.type_list();
                    } else {
                        p.ty();
                    }
                });
            }
            if p.at(T::Implements) {
                p.node(k::SUPER_INTERFACES, |p| {
                    p.bump_kw();
                    p.type_list();
                });
            }
            if p.at(T::Permits) {
                p.node(k::PERMITS, |p| {
                    p.bump_kw();
                    p.type_list();
                });
            }
            if kw == T::Enum {
                p.enum_body();
            } else {
                p.class_body(body);
            }
        });
    }

    fn record_header(&mut self) {
        self.node(k::RECORD_HEADER, |p| {
            p.expect(T::LParen, "`(`");
            p.list(T::RParen, |p| {
                p.node(k::RECORD_COMPONENT, |p| {
                    p.modifiers(k::MODIFIERS, false);
                    p.ty();
                    p.annotations();
                    p.eat(T::Ellipsis);
                    if p.at_name() {
                        p.bump();
                    } else {
                        p.error("expected a name");
                    }
                    p.dimensions();
                });
            });
        });
    }

    /// `{ members }` of a class, interface, record or annotation type (`kind` is the body node).
    pub(super) fn class_body(&mut self, kind: SyntaxKind) {
        let body = if kind == k::INTERFACE_BODY {
            Body::Interface
        } else if kind == k::RECORD_BODY {
            Body::Record
        } else if kind == k::ANNOTATION_TYPE_BODY {
            Body::Annotation
        } else {
            Body::Class
        };
        self.node(kind, |p| {
            p.expect(T::LBrace, "`{`");
            p.members(body);
            p.expect(T::RBrace, "`}`");
        });
    }

    fn members(&mut self, body: Body) {
        while !self.at(T::RBrace) && !self.at_eof() {
            let before = self.pos;
            deep(|| self.member(body));
            if self.pos == before {
                self.bump_error("unexpected token");
            }
        }
    }

    fn enum_body(&mut self) {
        self.node(k::ENUM_BODY, |p| {
            p.expect(T::LBrace, "`{`");
            while !p.at(T::Semi) && !p.at(T::RBrace) && !p.at_eof() {
                let before = p.pos;
                p.node(k::ENUM_CONSTANT, |p| {
                    p.modifiers(k::MODIFIERS, false);
                    if p.at_name() {
                        p.bump();
                    } else {
                        p.error("expected an enum constant");
                    }
                    if p.at(T::LParen) {
                        p.arguments();
                    }
                    if p.at(T::LBrace) {
                        p.class_body(k::CLASS_BODY);
                    }
                });
                if !p.eat(T::Comma) {
                    if p.pos == before {
                        p.bump_error("unexpected token");
                    }
                    break;
                }
            }
            if p.at(T::Semi) {
                p.node(k::ENUM_BODY_DECLARATIONS, |p| {
                    p.bump();
                    p.members(Body::Class);
                });
            }
            p.expect(T::RBrace, "`}`");
        });
    }

    /// One member of a body: a type, a field, a method, a constructor, an initializer or `;`.
    fn member(&mut self, body: Body) {
        if self.at(T::Semi) {
            self.bump();
            return;
        }
        if self.type_declaration() {
            return;
        }
        if self.at(T::LBrace) {
            self.block();
            return;
        }
        if self.at(T::Static) && self.nth(1) == T::LBrace {
            self.node(k::STATIC_INITIALIZER, |p| {
                p.bump_kw();
                p.block();
            });
            return;
        }
        let cp = self.cp();
        self.modifiers(k::MODIFIERS, true);
        if self.at(T::Lt) {
            self.type_parameters();
            self.annotations();
        }
        if self.at_name() && self.nth(1) == T::LParen && body != Body::Annotation {
            self.wrap(cp, k::CONSTRUCTOR_DECLARATION, |p| {
                p.bump();
                p.formal_parameters();
                p.throws();
                p.node(k::CONSTRUCTOR_BODY, |p| {
                    p.expect(T::LBrace, "`{`");
                    p.statements(true);
                    p.expect(T::RBrace, "`}`");
                });
            });
        } else if body == Body::Record && self.at_name() && self.nth(1) == T::LBrace {
            self.wrap(cp, k::COMPACT_CONSTRUCTOR_DECLARATION, |p| {
                p.bump();
                p.block();
            });
        } else if self.at(T::Void) || self.type_start() {
            if self.at(T::Void) {
                self.bump_kw();
            } else {
                self.ty();
            }
            if !self.at_name() {
                self.error("expected a name");
                self.recover_member();
            } else if self.nth(1) == T::LParen {
                if body == Body::Annotation && self.nth(2) == T::RParen {
                    self.wrap(cp, k::ANNOTATION_TYPE_ELEMENT_DECLARATION, |p| {
                        p.bump();
                        p.bump();
                        p.bump();
                        p.dimensions();
                        if p.at(T::Default) {
                            p.node(k::DEFAULT_VALUE, |p| {
                                p.bump_kw();
                                p.element_value();
                            });
                        }
                        p.semi();
                    });
                } else {
                    self.wrap(cp, k::METHOD_DECLARATION, |p| {
                        p.bump();
                        p.formal_parameters();
                        p.dimensions();
                        p.throws();
                        if p.at(T::LBrace) {
                            p.block();
                        } else if !p.eat(T::Semi) {
                            p.error("expected `{` or `;`");
                            p.recover_member();
                        }
                    });
                }
            } else {
                let kind = if body == Body::Interface || body == Body::Annotation {
                    k::CONSTANT_DECLARATION
                } else {
                    k::FIELD_DECLARATION
                };
                self.wrap(cp, kind, |p| {
                    p.declarators();
                    if !p.eat(T::Semi) {
                        p.error("expected `;`");
                        p.recover_member();
                    }
                });
            }
        } else {
            self.error("expected a member");
            self.recover_member();
        }
    }

    fn type_start(&self) -> bool {
        self.skip_type(self.pos, false).is_some()
    }

    /// After an error in a member: skips to the end of its `;` or block, or to the `}` that closes
    /// the body.
    fn recover_member(&mut self) {
        let mut depth = 0usize;
        while !self.at_eof() {
            match self.cur() {
                T::Semi if depth == 0 => {
                    self.bump();
                    return;
                }
                T::LBrace => depth += 1,
                T::RBrace => {
                    if depth == 0 {
                        return;
                    }
                    depth -= 1;
                    if depth == 0 {
                        self.bump();
                        return;
                    }
                }
                _ => {}
            }
            self.bump();
        }
    }

    // Parameters

    pub(super) fn formal_parameters(&mut self) {
        self.node(k::FORMAL_PARAMETERS, |p| {
            p.expect(T::LParen, "`(`");
            p.list(T::RParen, |p| p.formal_parameter());
        });
    }

    /// `final Type name`, `Type... name`, `Type this`.
    pub(super) fn formal_parameter(&mut self) {
        let cp = self.cp();
        self.modifiers(k::MODIFIERS, false);
        self.ty();
        self.annotations();
        if self.at(T::Ellipsis) {
            self.wrap(cp, k::SPREAD_PARAMETER, |p| {
                p.bump();
                if p.at_name() {
                    p.bump();
                } else {
                    p.error("expected a name");
                }
            });
        } else if self.at(T::This)
            || (self.at_name() && self.nth(1) == T::Dot && self.nth(2) == T::This)
        {
            self.wrap(cp, k::RECEIVER_PARAMETER, |p| {
                if !p.at(T::This) {
                    p.bump();
                    p.bump();
                }
                p.bump_kw();
            });
        } else {
            self.wrap(cp, k::FORMAL_PARAMETER, |p| {
                if p.at_name() {
                    p.bump();
                } else {
                    p.error("expected a name");
                }
                p.dimensions();
            });
        }
    }

    fn throws(&mut self) {
        if !self.at(T::Throws) {
            return;
        }
        self.node(k::THROWS, |p| {
            p.bump_kw();
            loop {
                p.node(k::EXCEPTION_TYPE, |p| {
                    p.annotations();
                    p.simple_type();
                });
                if !p.eat(T::Comma) {
                    return;
                }
            }
        });
    }
}
