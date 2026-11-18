//! What the declarations of a name say once the order of the code is known: imports nothing
//! read, definitions replaced before a read, imports covered by a loop variable, local variables
//! never read, arguments rebound by a local.
//!
//! concept: ordered-scope-rules
//! what: Scope rules where code order decides what a name means: replaced definitions, covered imports, unread locals.
//! tools: pyflakes, ruff F811 F402 F841
//! reads: family:scope

use super::Hit;
use super::imports::Form;
use super::live::{Binding, Bindings, Kind, Live, Lives, is_dummy, own};
use super::names::qualified;
use crate::scope_rules::cx::Cx;
use core_semantics::scopes::{Flags, ScopeId, SymbolId};
use core_syntax::TextRange;
use std::collections::HashSet;

fn scope_symbols<'a>(cx: &'a Cx<'a>, scope: ScopeId) -> impl Iterator<Item = SymbolId> + 'a {
    cx.m.scope(scope).symbols.iter().copied()
}

/// An import that nothing reads.
pub(super) fn unused_imports(cx: &Cx, lv: &Lives<'_>, exports: &[(String, TextRange)]) -> Vec<Hit> {
    let exported: HashSet<&str> = exports.iter().map(|(n, _)| n.as_str()).collect();
    let mut hits = Vec::new();
    for (i, scope) in cx.m.scopes.iter().enumerate() {
        let id = ScopeId(i as u32);
        if cx.tag(&scope.node, "type-declaration") {
            continue;
        }
        for sym in scope_symbols(cx, id) {
            let s = cx.m.symbol(sym);
            if !s.flags.contains(Flags::IMPORT) || &*s.name == "*" {
                continue;
            }
            let Some(last) = lv.get(sym).last() else {
                continue;
            };
            let Some(import) = &last.b.import else {
                continue;
            };
            if last.used
                || import.explicit
                || import.form == Form::Pragma
                || (i == 0 && exported.contains(&*s.name))
                || (import.form == Form::Submodule && alias_used(cx, lv, id, &import.target))
            {
                continue;
            }
            hits.push((last.b.range, import.label.clone()));
        }
    }
    hits
}

/// `import a.b` next to a used `import a as x`: reading `x` reaches `a.b` too.
fn alias_used(cx: &Cx, lv: &Lives<'_>, scope: ScopeId, target: &str) -> bool {
    scope_symbols(cx, scope).any(|sym| {
        lv.get(sym).last().is_some_and(|l| {
            l.used
                && l.b.kind == Kind::Import(Form::Import)
                && l.b.import.as_ref().is_some_and(|i| i.target == target)
        })
    })
}

/// Does a new binding of `b` redefine the definition `old`?
fn redefines(b: &Binding, old: &Binding) -> bool {
    match b.kind {
        Kind::Annotation | Kind::Global | Kind::Import(Form::Pragma) => false,
        Kind::Import(form) => match (form, old.kind) {
            (Form::Import | Form::From, Kind::Import(Form::Submodule))
            | (Form::Submodule, Kind::Import(Form::Import | Form::Submodule)) => {
                label(b) == label(old)
            }
            _ => true,
        },
        _ => true,
    }
}

fn label(b: &Binding) -> &str {
    b.import.as_ref().map_or("", |i| i.label.as_str())
}

/// Is the function decorated with one of the language's overload markers?
fn is_overload(cx: &Cx, l: &Live<'_>) -> bool {
    l.b.kind == Kind::FunctionDef
        && cx.decl_node(l.decl).children().any(|d| {
            cx.tag(&d, "decorator")
                && d.first_child()
                    .and_then(|e| qualified(cx, &e))
                    .is_some_and(|q| cx.ordered().overloads.contains(&q.as_str()))
        })
}

/// A definition replaced, or covered by a name of a function, before anything read it (the first
/// list); an import replaced by a loop variable (the second).
pub(super) fn redefinitions(cx: &Cx, lv: &Lives<'_>) -> (Vec<Hit>, Vec<Hit>) {
    let (mut redefined, mut loops) = (Vec::new(), Vec::new());
    for id in cx.m.symbol_ids() {
        let sym = cx.m.symbol(id);
        if &*sym.name == "*" {
            continue;
        }
        if sym.decls.len() > 1 {
            for pair in lv.get(id).windows(2) {
                let (old, new) = (&pair[0], &pair[1]);
                if old.b.branch != new.b.branch {
                    continue;
                }
                let was_import = old.b.kind.is_import();
                if new.b.kind == Kind::LoopVar && was_import {
                    loops.push((new.b.range, sym.name.to_string()));
                    continue;
                }
                let definition = was_import
                    || matches!(old.b.kind, Kind::FunctionDef | Kind::ClassDef)
                    || (matches!(old.b.kind, Kind::Assignment | Kind::NamedExpr)
                        && (new.b.kind.is_import()
                            || matches!(new.b.kind, Kind::FunctionDef | Kind::ClassDef)));
                if old.used
                    || !definition
                    || old.b.kind == Kind::Import(Form::Pragma)
                    || is_overload(cx, old)
                {
                    continue;
                }
                if redefines(new.b, old.b) && (!is_dummy(cx, &sym.name) || was_import) {
                    redefined.push((new.b.range, sym.name.to_string()));
                }
            }
        }
        shadow(cx, lv, id, &mut redefined, &mut loops);
    }
    (redefined, loops)
}

/// A first binding in a function's scope (or a comprehension's) of a name an import of an
/// enclosing scope has: reported when nothing read the import.
fn shadow(cx: &Cx, lv: &Lives<'_>, id: SymbolId, redefined: &mut Vec<Hit>, loops: &mut Vec<Hit>) {
    let m = cx.m;
    let sym = m.symbol(id);
    if cx.tag(&m.scope(sym.scope).node, "type-declaration")
        || sym.scope == ScopeId(0)
        || !sym.flags.contains(Flags::VALUE)
    {
        return;
    }
    let Some(first) = sym.decls.first() else {
        return;
    };
    if !own(cx, sym.scope, first) || first.flags.contains(Flags::TYPE_PARAMETER) {
        return;
    }
    let by = &lv.bs.get(id)[0];
    if matches!(
        by.kind,
        Kind::Annotation | Kind::Global | Kind::Import(Form::Pragma)
    ) || by.branch.is_some()
    {
        return;
    }
    let outer = m
        .ancestors(sym.scope)
        .skip(1)
        .find_map(|s| m.own(s, &sym.name));
    let Some(outer) = outer else { return };
    if !m.symbol(outer).flags.contains(Flags::IMPORT) {
        return;
    }
    let Some(old) = lv.get(outer).last() else {
        return;
    };
    if !old.b.kind.is_import() || old.b.branch.is_some() || old.b.kind == Kind::Import(Form::Pragma)
    {
        return;
    }
    if by.kind == Kind::LoopVar {
        loops.push((first.name, sym.name.to_string()));
    } else if !old.used && redefines(by, old.b) {
        redefined.push((first.name, sym.name.to_string()));
    }
}

/// A `for`, `with` or `except` name that is also an argument of the function and has not been
/// bound since.
pub(super) fn redefined_arguments(cx: &Cx, bs: &Bindings<'_>) -> Vec<Hit> {
    let mut hits = Vec::new();
    for id in cx.m.symbol_ids() {
        let bindings = bs.get(id);
        if !bindings.iter().any(|b| b.kind == Kind::Argument) {
            continue;
        }
        let sym = cx.m.symbol(id);
        // Only while the name still holds the argument: after another binding it holds a local.
        for (i, (d, b)) in sym.decls.iter().zip(bindings).enumerate() {
            if matches!(b.kind, Kind::LoopVar | Kind::WithItem | Kind::Except)
                && bindings[..i].iter().all(|p| p.kind == Kind::Argument)
            {
                hits.push((d.name, sym.name.to_string()));
            }
        }
    }
    hits
}

/// Which names `unused_locals` looks for.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Local {
    Variable,
    Annotation,
    /// A name of a tuple target.
    Unpacked,
}

/// A local variable assigned or annotated and never read, and a name an `except` clause binds
/// and its body never reads.
pub(super) fn unused_locals(cx: &Cx, lv: &Lives<'_>, mode: Local) -> Vec<Hit> {
    let m = cx.m;
    let annotation = mode == Local::Annotation;
    let mut hits = Vec::new();
    for id in m.symbol_ids() {
        let sym = m.symbol(id);
        let ok = is_dummy(cx, &sym.name);
        if !sym.flags.contains(Flags::VALUE) || ok && !sym.flags.contains(Flags::CATCH) {
            continue;
        }
        if mode == Local::Variable && sym.flags.contains(Flags::CATCH) {
            for d in sym.decls.iter().filter(|d| d.flags.contains(Flags::CATCH)) {
                let handler = d.node_range;
                let read = m
                    .refs_of(id)
                    .any(|r| r.is_read() && handler.contains_range(r.range));
                if !read {
                    hits.push((d.name, sym.name.to_string()));
                }
            }
        }
        let scope = m.scope(sym.scope);
        if !cx.is_function(&scope.node) || scope.dynamic || ok {
            continue;
        }
        let Some(last) = lv.get(id).last() else {
            continue;
        };
        let kind_ok = if annotation {
            last.b.kind == Kind::Annotation
        } else {
            matches!(
                last.b.kind,
                Kind::Assignment | Kind::NamedExpr | Kind::WithItem
            ) && (last.b.unpacked == (mode == Local::Unpacked))
        };
        if kind_ok && !last.used {
            hits.push((last.decl.name, sym.name.to_string()));
        }
    }
    hits
}
