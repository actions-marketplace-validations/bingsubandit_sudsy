//! Statements, `switch` and patterns.

use super::P;
use crate::generated::kind as k;
use crate::lexer::T;
use core_format::deep;

impl P<'_> {
    pub(super) fn block(&mut self) {
        self.node(k::BLOCK, |p| {
            p.expect(T::LBrace, "`{`");
            p.statements(false);
            p.expect(T::RBrace, "`}`");
        });
    }

    /// Statements up to a `}`; in a switch group also up to the next `case` or `default`.
    /// `constructor`: the body of a constructor, where `this(...)` and `super(...)` may start.
    pub(super) fn statements(&mut self, constructor: bool) {
        while !self.at(T::RBrace) && !self.at_eof() {
            let before = self.pos;
            if constructor && self.explicit_constructor_invocation() {
                continue;
            }
            self.statement();
            if self.pos == before {
                self.bump_error("unexpected token");
            }
        }
    }

    fn group_statements(&mut self) {
        while !matches!(self.cur(), T::RBrace | T::Case | T::Default) && !self.at_eof() {
            let before = self.pos;
            self.statement();
            if self.pos == before {
                self.bump_error("unexpected token");
            }
        }
    }

    /// `this(...)`, `super(...)`, `<T>this(...)` as the first statement of a constructor.
    fn explicit_constructor_invocation(&mut self) -> bool {
        if self.qualified_super_ahead() {
            self.node(k::EXPLICIT_CONSTRUCTOR_INVOCATION, |p| {
                p.postfix();
                p.bump();
                if p.at(T::Lt) {
                    p.type_arguments();
                }
                p.bump_kw();
                p.arguments();
                p.semi();
            });
            return true;
        }
        let mut j = self.pos;
        if self.at(T::Lt) {
            match self.skip_type_args(j, 0) {
                Some(e) => j = e,
                None => return false,
            }
        }
        if !matches!(self.at_idx(j), T::This | T::Super) || self.at_idx(j + 1) != T::LParen {
            return false;
        }
        self.node(k::EXPLICIT_CONSTRUCTOR_INVOCATION, |p| {
            if p.at(T::Lt) {
                p.type_arguments();
            }
            p.bump_kw();
            p.arguments();
            p.semi();
        });
        true
    }

    /// `outer.super(...)`: a `.super(` at the top level of the statement.
    fn qualified_super_ahead(&self) -> bool {
        let mut depth = 0usize;
        for i in self.pos..self.toks.len() {
            match self.at_idx(i) {
                T::LParen | T::LBracket | T::LBrace => depth += 1,
                T::RParen | T::RBracket | T::RBrace => {
                    if depth == 0 {
                        return false;
                    }
                    depth -= 1;
                }
                T::Semi if depth == 0 => return false,
                T::Dot
                    if depth == 0
                        && self.at_idx(i + 1) == T::Super
                        && self.at_idx(i + 2) == T::LParen =>
                {
                    return true;
                }
                _ => {}
            }
        }
        false
    }

    /// The `;` that ends a statement; without it the error is reported and the next statement
    /// starts at the current token.
    pub(super) fn semi(&mut self) {
        self.expect(T::Semi, "`;`");
    }

    pub(super) fn paren_expr(&mut self) {
        self.node(k::PARENTHESIZED_EXPRESSION, |p| {
            p.expect(T::LParen, "`(`");
            p.nested_expr();
            p.expect(T::RParen, "`)`");
        });
    }

    pub(super) fn statement(&mut self) {
        deep(|| self.statement_inner())
    }

    fn statement_inner(&mut self) {
        match self.cur() {
            T::LBrace => self.block(),
            T::Semi => self.bump(),
            T::If => self.node(k::IF_STATEMENT, |p| {
                p.bump_kw();
                p.paren_expr();
                p.statement();
                if p.at(T::Else) {
                    p.bump_kw();
                    p.statement();
                }
            }),
            T::While => self.node(k::WHILE_STATEMENT, |p| {
                p.bump_kw();
                p.paren_expr();
                p.statement();
            }),
            T::Do => self.node(k::DO_STATEMENT, |p| {
                p.bump_kw();
                p.statement();
                if p.at(T::While) {
                    p.bump_kw();
                } else {
                    p.error("expected `while`");
                }
                p.paren_expr();
                p.semi();
            }),
            T::For => self.for_statement(),
            T::Try => self.try_statement(),
            T::Switch => self.switch(false),
            T::Synchronized if self.nth(1) == T::LParen => {
                self.node(k::SYNCHRONIZED_STATEMENT, |p| {
                    p.bump_kw();
                    p.paren_expr();
                    p.block();
                })
            }
            T::Return => self.node(k::RETURN_STATEMENT, |p| {
                p.bump_kw();
                if !p.at(T::Semi) {
                    p.expr();
                }
                p.semi();
            }),
            T::Throw => self.node(k::THROW_STATEMENT, |p| {
                p.bump_kw();
                p.expr();
                p.semi();
            }),
            T::Break => self.node(k::BREAK_STATEMENT, |p| p.jump()),
            T::Continue => self.node(k::CONTINUE_STATEMENT, |p| p.jump()),
            T::Assert => self.node(k::ASSERT_STATEMENT, |p| {
                p.bump_kw();
                p.expr();
                if p.at(T::Colon) {
                    p.bump();
                    p.expr();
                }
                p.semi();
            }),
            T::Yield if self.yield_ahead() => self.node(k::YIELD_STATEMENT, |p| {
                p.bump_kw();
                p.expr();
                p.semi();
            }),
            t if t.is_name() && self.nth(1) == T::Colon => self.node(k::LABELED_STATEMENT, |p| {
                p.node(k::LABEL, |p| p.bump());
                p.bump();
                p.statement();
            }),
            T::Else | T::Case | T::Default | T::Catch | T::Finally => {
                // Left to the construct that owns it; as a statement it is an error.
                self.bump_error("unexpected keyword");
            }
            _ => self.declaration_or_expression(),
        }
    }

    /// `yield expr;`: a name after `yield` that is not an operator or the start of a member access.
    fn yield_ahead(&self) -> bool {
        !matches!(
            self.nth(1),
            T::Eq
                | T::AssignOp
                | T::Dot
                | T::LBracket
                | T::PlusPlus
                | T::MinusMinus
                | T::Semi
                | T::Colon
        )
    }

    /// `break label;`, `continue;`.
    fn jump(&mut self) {
        self.bump_kw();
        if self.at_name() {
            self.node(k::LABEL, |p| p.bump());
        }
        self.semi();
    }

    /// A local variable, a local type or an expression statement.
    fn declaration_or_expression(&mut self) {
        let j = self.skip_modifiers(self.pos, false);
        let t = self.at_idx(j);
        let type_decl = matches!(t, T::Class | T::Interface | T::Enum | T::AtInterface)
            || (t == T::Record
                && self.at_idx(j + 1).is_name()
                && matches!(self.at_idx(j + 2), T::LParen | T::Lt));
        if type_decl {
            self.node(k::LOCAL_TYPE_DECLARATION, |p| {
                p.modifiers(k::LOCAL_TYPE_MODIFIERS, false);
                p.type_declaration_core();
            });
        } else if j != self.pos || self.variable_declaration_ahead(j) {
            self.node(k::LOCAL_VARIABLE_DECLARATION, |p| {
                p.node(k::LOCAL_VARIABLE_DECLARATION_CORE, |p| {
                    p.local_variable_core()
                });
                p.semi();
            });
        } else {
            self.node(k::EXPRESSION_STATEMENT, |p| {
                p.node(k::STATEMENT_EXPRESSION, |p| p.expr());
                if !p.at(T::Semi) {
                    p.error("expected `;`");
                    p.recover_statement();
                } else {
                    p.bump();
                }
            });
        }
    }

    /// After a bad expression statement: skips to the next `;` (consumed) or `}`, but stops at a
    /// token that starts a statement on its own so one error does not swallow the following ones.
    fn recover_statement(&mut self) {
        let mut depth = 0usize;
        let first = self.pos;
        while !self.at_eof() {
            let t = self.cur();
            if depth == 0 && self.pos > first && is_statement_keyword(t) {
                return;
            }
            match t {
                T::Semi if depth == 0 => {
                    self.bump();
                    return;
                }
                T::LBrace | T::LParen | T::LBracket => depth += 1,
                T::RBrace | T::RParen | T::RBracket => {
                    if depth == 0 {
                        return;
                    }
                    depth -= 1;
                }
                _ => {}
            }
            self.bump();
        }
    }

    /// Whether a type and a name follow at `j`: a declaration.
    pub(super) fn variable_declaration_ahead(&self, j: usize) -> bool {
        match self.skip_type(j, true) {
            Some(e) => self.at_idx(e).is_name(),
            None => false,
        }
    }

    /// `final Type a = 1, b` or `var a = 1`, without the `;`.
    pub(super) fn local_variable_core(&mut self) {
        self.modifiers(k::MODIFIERS, false);
        self.local_type();
        self.declarators();
    }

    /// `var`, when it is the whole type, or a type.
    pub(super) fn local_type(&mut self) {
        if self.at(T::Var) && self.nth(1).is_name() {
            self.node(k::INFERRED_LOCAL_TYPE, |p| p.bump_kw());
        } else {
            self.ty();
        }
    }

    /// `a = 1, b[] = {2}`.
    pub(super) fn declarators(&mut self) {
        loop {
            self.node(k::VARIABLE_DECLARATOR, |p| {
                if p.at_name() {
                    p.bump();
                } else {
                    p.error("expected a name");
                }
                p.dimensions();
                if p.at(T::Eq) {
                    p.bump();
                    if p.at(T::LBrace) {
                        p.array_initializer();
                    } else {
                        p.expr();
                    }
                }
            });
            if self.at(T::Comma) {
                self.bump();
            } else {
                return;
            }
        }
    }

    fn for_statement(&mut self) {
        let enhanced = self.enhanced_for_ahead();
        self.node(
            if enhanced {
                k::ENHANCED_FOR_STATEMENT
            } else {
                k::FOR_STATEMENT
            },
            |p| {
                p.bump_kw();
                p.node(k::FOR_SPEC, |p| {
                    p.expect(T::LParen, "`(`");
                    if enhanced {
                        p.modifiers(k::MODIFIERS, false);
                        p.local_type();
                        p.bump();
                        p.dimensions();
                        p.expect(T::Colon, "`:`");
                        p.expr();
                    } else {
                        if !p.at(T::Semi) {
                            p.node(k::FOR_INITIALIZER, |p| {
                                let j = p.skip_modifiers(p.pos, false);
                                if j != p.pos || p.variable_declaration_ahead(j) {
                                    p.node(k::LOCAL_VARIABLE_DECLARATION_CORE, |p| {
                                        p.local_variable_core()
                                    });
                                } else {
                                    p.statement_expressions();
                                }
                            });
                        }
                        p.expect(T::Semi, "`;`");
                        if !p.at(T::Semi) {
                            p.node(k::FOR_CONDITION, |p| p.expr());
                        }
                        p.expect(T::Semi, "`;`");
                        if !p.at(T::RParen) {
                            p.node(k::FOR_UPDATE, |p| p.statement_expressions());
                        }
                    }
                    p.expect(T::RParen, "`)`");
                });
                p.statement();
            },
        );
    }

    fn statement_expressions(&mut self) {
        loop {
            self.node(k::STATEMENT_EXPRESSION, |p| p.expr());
            if !self.eat(T::Comma) {
                return;
            }
        }
    }

    /// `for (Type name : ...`.
    fn enhanced_for_ahead(&self) -> bool {
        if self.nth(1) != T::LParen {
            return false;
        }
        let mut j = self.skip_modifiers(self.pos + 2, false);
        if self.at_idx(j) == T::Var && self.at_idx(j + 1).is_name() {
            j += 1;
        } else {
            match self.skip_type(j, true) {
                Some(e) => j = e,
                None => return false,
            }
        }
        if !self.at_idx(j).is_name() {
            return false;
        }
        j += 1;
        while self.at_idx(j) == T::LBracket && self.at_idx(j + 1) == T::RBracket {
            j += 2;
        }
        self.at_idx(j) == T::Colon
    }

    fn try_statement(&mut self) {
        let resources = self.nth(1) == T::LParen;
        self.node(
            if resources {
                k::TRY_WITH_RESOURCES_STATEMENT
            } else {
                k::TRY_STATEMENT
            },
            |p| {
                p.bump_kw();
                if resources {
                    p.node(k::RESOURCE_SPECIFICATION, |p| {
                        p.bump();
                        while !p.at(T::RParen) && !p.at_eof() {
                            let before = p.pos;
                            p.resource();
                            if !p.eat(T::Semi) {
                                if p.pos == before {
                                    p.error("expected a resource");
                                }
                                break;
                            }
                        }
                        p.expect(T::RParen, "`)`");
                    });
                }
                p.block();
                while p.at(T::Catch) {
                    p.node(k::CATCH_CLAUSE, |p| {
                        p.bump_kw();
                        p.catch_parameter();
                        p.block();
                    });
                }
                if p.at(T::Finally) {
                    p.node(k::FINALLY_CLAUSE, |p| {
                        p.bump_kw();
                        p.block();
                    });
                }
            },
        );
    }

    fn resource(&mut self) {
        let j = self.skip_modifiers(self.pos, false);
        let declaration = if self.at_idx(j) == T::Var && self.at_idx(j + 1).is_name() {
            self.at_idx(j + 2) == T::Eq
        } else {
            self.skip_type(j, true)
                .is_some_and(|e| self.at_idx(e).is_name() && self.at_idx(e + 1) == T::Eq)
        };
        self.node(k::RESOURCE, |p| {
            if declaration {
                p.modifiers(k::MODIFIERS, false);
                p.local_type();
                p.bump();
                p.expect(T::Eq, "`=`");
                p.expr();
            } else {
                p.expr();
            }
        });
    }

    fn catch_parameter(&mut self) {
        self.node(k::CATCH_FORMAL_PARAMETER, |p| {
            p.expect(T::LParen, "`(`");
            p.modifiers(k::MODIFIERS, false);
            p.node(k::CATCH_TYPE, |p| {
                p.simple_type();
                while p.at(T::Pipe) {
                    p.bump_as(k::OPERATOR);
                    p.node(k::CATCH_ALTERNATIVE_TYPE, |p| {
                        p.annotations();
                        p.simple_type();
                    });
                }
            });
            if p.at_name() {
                p.bump();
            } else {
                p.error("expected a name");
            }
            p.expect(T::RParen, "`)`");
        });
    }

    // switch

    /// A `switch` statement, or an expression when `expression`.
    pub(super) fn switch(&mut self, expression: bool) {
        let (kind, block) = if expression {
            (k::SWITCH_EXPRESSION, k::SWITCH_EXPRESSION_BLOCK)
        } else {
            (k::SWITCH_STATEMENT, k::SWITCH_STATEMENT_BLOCK)
        };
        self.node(kind, |p| {
            p.bump_kw();
            p.paren_expr();
            p.node(block, |p| {
                p.expect(T::LBrace, "`{`");
                while !p.at(T::RBrace) && !p.at_eof() {
                    let before = p.pos;
                    p.switch_entry(expression);
                    if p.pos == before {
                        p.bump_error("expected `case` or `default`");
                    }
                }
                p.expect(T::RBrace, "`}`");
            });
        });
    }

    /// `case ... ->` rule or `case ...:` group.
    fn switch_entry(&mut self, expression: bool) {
        if !matches!(self.cur(), T::Case | T::Default) {
            return;
        }
        let cp = self.cp();
        self.switch_label();
        if self.at(T::Arrow) {
            let kind = if expression {
                k::SWITCH_EXPRESSION_RULE
            } else {
                k::SWITCH_STATEMENT_RULE
            };
            self.wrap(cp, kind, |p| {
                p.bump();
                match p.cur() {
                    T::LBrace => p.block(),
                    T::Throw => p.statement(),
                    _ if expression => {
                        p.expr();
                        p.semi();
                    }
                    _ => {
                        p.node(k::STATEMENT_EXPRESSION, |p| p.expr());
                        p.semi();
                    }
                }
            });
        } else {
            self.wrap(cp, k::SWITCH_BLOCK_STATEMENT_GROUP, |p| {
                p.expect(T::Colon, "`:` or `->`");
                if !matches!(p.cur(), T::RBrace | T::Case | T::Default) {
                    p.node(k::SWITCH_BLOCK_STATEMENT_GROUP_BODY, |p| {
                        p.node(k::SWITCH_GROUP_STATEMENTS, |p| p.group_statements());
                    });
                }
            });
        }
    }

    fn switch_label(&mut self) {
        self.node(k::SWITCH_LABEL, |p| {
            if p.at(T::Default) {
                p.bump_kw();
                return;
            }
            p.bump_kw();
            let lambda = std::mem::replace(&mut p.no_lambda, true);
            p.node(k::SWITCH_CASE_LABEL_REST, |p| {
                loop {
                    p.node(k::SWITCH_CASE_ELEMENT, |p| {
                        if p.at(T::Default) {
                            p.bump_kw();
                        } else if p.pattern_ahead() {
                            p.pattern();
                        } else {
                            p.ternary();
                        }
                    });
                    if !p.eat(T::Comma) {
                        break;
                    }
                }
                if p.at(T::When) {
                    p.node(k::GUARD, |p| {
                        p.bump_kw();
                        p.expr();
                    });
                }
            });
            p.no_lambda = lambda;
        });
    }

    /// Whether the case element at the current token is a pattern: a type followed by a name or
    /// by the components of a record pattern.
    fn pattern_ahead(&self) -> bool {
        let j = self.skip_modifiers(self.pos, false);
        if j != self.pos {
            return true;
        }
        match self.skip_type(j, true) {
            Some(e) => self.at_idx(e).is_name() || self.at_idx(e) == T::LParen,
            None => false,
        }
    }

    /// `Type name`, `Type(components)`, `final Type name`.
    fn pattern(&mut self) {
        self.node(k::PATTERN_NODE, |p| {
            p.modifier_tokens(false);
            p.ty();
            if p.at(T::LParen) {
                p.record_pattern_body();
                if p.at_name() && p.cur() != T::When {
                    p.bump();
                }
            } else if p.at_name() {
                p.node(k::PATTERN_VARIABLE, |p| p.bump());
            } else {
                p.error("expected a pattern variable");
            }
        });
    }

    /// `(a, b)` of a record pattern: patterns and `_`.
    pub(super) fn record_pattern_body(&mut self) {
        self.bump();
        self.list(T::RParen, |p| {
            if p.src_is_underscore() && !matches!(p.nth(1), T::Dot | T::Lt) {
                p.bump();
            } else {
                deep(|| p.pattern());
            }
        });
    }

    fn src_is_underscore(&self) -> bool {
        self.toks
            .get(self.pos)
            .is_some_and(|t| &self.src[t.start as usize..t.end as usize] == "_")
    }
}

/// Keywords that can only start a statement.
fn is_statement_keyword(t: T) -> bool {
    matches!(
        t,
        T::If
            | T::For
            | T::While
            | T::Do
            | T::Try
            | T::Return
            | T::Throw
            | T::Break
            | T::Continue
            | T::Switch
            | T::Assert
            | T::Else
            | T::Case
            | T::Default
            | T::Catch
            | T::Finally
    )
}
