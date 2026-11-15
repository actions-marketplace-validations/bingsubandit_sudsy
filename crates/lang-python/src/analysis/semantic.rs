//! ruff's semantic syntax errors (`return` outside a function, duplicate parameters, ...): the
//! checker of `ruff_python_parser` asks the scopes of the walk what it needs. Errors that have
//! a rule of their own in ruff's default set (F404, F407, F622, F701, F702, F704, F706) report as
//! that rule, the others as `invalid-syntax`.

use super::checker::Checker;
use super::model::{Kind, ScopeKind};
use ruff_python_ast::{Expr, PythonVersion, Stmt};
use ruff_python_parser::semantic_errors::{
    LazyImportContext, SemanticSyntaxContext, SemanticSyntaxError, SemanticSyntaxErrorKind as E,
};
use ruff_text_size::TextRange;

impl Checker<'_> {
    pub(super) fn semantic_stmt(&mut self, stmt: &Stmt) {
        let mut sem = std::mem::take(&mut self.sem);
        sem.visit_stmt(stmt, &*self);
        self.sem = sem;
        self.drain_semantic();
    }

    pub(super) fn semantic_expr(&mut self, expr: &Expr) {
        let mut sem = std::mem::take(&mut self.sem);
        sem.visit_expr(expr, &*self);
        self.sem = sem;
        self.drain_semantic();
    }

    fn drain_semantic(&mut self) {
        let errors: Vec<SemanticSyntaxError> = self.sem_errors.borrow_mut().drain(..).collect();
        for e in errors {
            let (check, name) = match &e.kind {
                E::LateFutureImport => ("late-future-import", String::new()),
                E::FutureFeatureNotDefined(f) => ("future-feature-not-defined", f.clone()),
                E::MultipleStarredExpressions => ("multiple-starred-expressions", String::new()),
                E::ReturnOutsideFunction => ("return-outside-function", String::new()),
                E::YieldOutsideFunction(_) => ("yield-outside-function", String::new()),
                E::BreakOutsideLoop => ("break-outside-loop", String::new()),
                E::ContinueOutsideLoop => ("continue-outside-loop", String::new()),
                E::NonModuleImportStar(_) => continue,
                _ => ("invalid-syntax", e.to_string()),
            };
            self.m.hit(check, e.range, name);
        }
    }
}

impl SemanticSyntaxContext for Checker<'_> {
    fn future_annotations_or_stub(&self) -> bool {
        self.future_annotations || self.stub
    }

    fn lazy_import_context(&self) -> Option<LazyImportContext> {
        None
    }

    fn python_version(&self) -> PythonVersion {
        self.target
    }

    fn source(&self) -> &str {
        self.src
    }

    fn global(&self, name: &str) -> Option<TextRange> {
        let scope = &self.m.scopes[self.m.current()];
        scope.declared.get(name).filter(|d| d.0).map(|d| d.1)
    }

    fn has_nonlocal_binding(&self, name: &str) -> bool {
        let mut at = self.m.scopes[self.m.current()].parent;
        while let Some(s) = at {
            let scope = &self.m.scopes[s];
            if scope.kind.is_function() && scope.bindings.contains_key(name) {
                return true;
            }
            at = scope.parent;
        }
        false
    }

    fn in_async_context(&self) -> bool {
        for &s in self.m.stack.iter().rev() {
            match self.m.scopes[s].kind {
                ScopeKind::Class | ScopeKind::Lambda => return false,
                ScopeKind::Function { is_async } => return is_async,
                _ => {}
            }
        }
        false
    }

    fn in_await_allowed_context(&self) -> bool {
        for &s in self.m.stack.iter().rev() {
            match self.m.scopes[s].kind {
                ScopeKind::Class => return false,
                ScopeKind::Function { .. } | ScopeKind::Lambda => return true,
                _ => {}
            }
        }
        false
    }

    fn in_yield_allowed_context(&self) -> bool {
        for &s in self.m.stack.iter().rev() {
            match self.m.scopes[s].kind {
                ScopeKind::Class | ScopeKind::Generator { .. } => return false,
                ScopeKind::Function { .. } | ScopeKind::Lambda => return true,
                _ => {}
            }
        }
        false
    }

    fn in_sync_comprehension(&self) -> bool {
        self.m.stack.iter().any(|&s| {
            matches!(
                self.m.scopes[s].kind,
                ScopeKind::Generator {
                    is_gen: false,
                    is_async: false
                }
            )
        })
    }

    fn in_class_body_comprehension(&self) -> bool {
        for &s in self.m.stack.iter().rev() {
            match self.m.scopes[s].kind {
                ScopeKind::Generator { .. } | ScopeKind::Type => {}
                ScopeKind::Class => return true,
                _ => return false,
            }
        }
        false
    }

    fn in_module_scope(&self) -> bool {
        self.m.current() == 0
    }

    fn in_function_scope(&self) -> bool {
        self.m.scopes[self.m.current()].kind.is_function()
    }

    fn in_generator_context(&self) -> bool {
        self.m.stack.iter().any(|&s| {
            matches!(
                self.m.scopes[s].kind,
                ScopeKind::Generator { is_gen: true, .. }
            )
        })
    }

    fn in_notebook(&self) -> bool {
        false
    }

    fn report_semantic_error(&self, error: SemanticSyntaxError) {
        self.sem_errors.borrow_mut().push(error);
    }

    fn in_loop_context(&self) -> bool {
        self.loop_depth > 0
    }

    fn is_bound_parameter(&self, name: &str) -> bool {
        let scope = &self.m.scopes[self.m.current()];
        scope.kind.is_function()
            && scope
                .bindings
                .get(name)
                .is_some_and(|&b| self.m.bindings[b].kind == Kind::Argument)
    }
}
