//! Reads of names: those no scope binds, those that run before the name is bound, the names a
//! module exports.
//!
//! concept: ordered-scope-rules
//! what: Scope rules where code order decides what a name means: replaced definitions, covered imports, unread locals.
//! tools: pyflakes, ruff F811 F402 F841
//! reads: family:scope

use super::live::{Bindings, Kind, deferred, own};
use super::{Hit, imports::Form};
use crate::scope_rules::cx::Cx;
use core_semantics::scopes::{Flags, ScopeId, SymbolId};
use core_syntax::roles::{code_text, has_own_token};
use core_syntax::{SyntaxNode, TextRange, TextSize};
use std::collections::HashMap;

/// The names of the strings an assignment to the export list at module level names (earlier
/// assignments count too).
pub(super) fn exports(cx: &Cx) -> Vec<(String, TextRange)> {
    fn collect(cx: &Cx, e: &SyntaxNode, out: &mut Vec<(String, TextRange)>) {
        if cx.tag(e, "collection") || (cx.tag(e, "binary") && has_own_token(e, "+")) {
            e.children().for_each(|c| collect(cx, &c, out));
        } else if cx.tag(e, "string-literal") {
            out.push((string_value(e), cx.range(e)));
        }
    }
    let mut out = Vec::new();
    let name = cx.ordered().exports;
    let Some(id) = cx.m.own(ScopeId(0), name).filter(|_| !name.is_empty()) else {
        return out;
    };
    for d in &cx.m.symbol(id).decls {
        let target = cx.binding(d);
        let Some(stmt) = target.parent().filter(|s| cx.tag(s, "assignment")) else {
            continue;
        };
        if let Some(v) = cx.slot(&stmt, "right").filter(|v| *v != target) {
            collect(cx, &v, &mut out);
        }
    }
    out
}

/// The value of the plain string literal, its parts joined.
fn string_value(e: &SyntaxNode) -> String {
    e.children()
        .map(|part| {
            let text = code_text(&part);
            let body = text.trim_start_matches(|c: char| c.is_ascii_alphabetic());
            let quote = body.get(..1).unwrap_or("");
            let width = if body.len() >= 6 && body.starts_with(&quote.repeat(3)) {
                3
            } else {
                1
            };
            body.get(width..body.len().saturating_sub(width))
                .unwrap_or("")
                .to_owned()
        })
        .collect()
}

/// The names a dotted expression stands for through imports: `typing.overload` for `overload`
/// imported from typing.
pub(super) fn qualified(cx: &Cx, n: &SyntaxNode) -> Option<String> {
    if cx.tag(n, "reference") {
        let start = cx.range(n).start();
        let r = cx.m.refs.iter().find(|r| r.range.start() == start)?;
        let d = cx.m.symbol(r.symbol?).decls.last()?;
        super::imports::describe(cx.file.grammar, cx.root(), cx.ordered().pragma, d)
            .map(|i| i.target)
    } else if cx.tag(n, "member-access") {
        let (object, member) = (cx.slot(n, "object")?, cx.slot(n, "member")?);
        Some(format!(
            "{}.{}",
            qualified(cx, &object)?,
            code_text(&member)
        ))
    } else if cx.tag(n, "transparent") {
        qualified(cx, &n.first_child()?)
    } else {
        None
    }
}

/// Is the name read where a `try` that handles the guard exception is running its body?
fn guarded(cx: &Cx, at: &SyntaxNode) -> bool {
    let guard = cx.ordered().guard;
    let mut child = at.clone();
    for a in at.ancestors().skip(1) {
        if cx.is_function(&a) || cx.tag(&a, "type-declaration") {
            return false;
        }
        if cx.tag(&a, "try") {
            let handlers: Vec<SyntaxNode> = a.children().filter(|h| cx.tag(h, "catch")).collect();
            let in_body = handlers
                .first()
                .is_none_or(|h| child.text_range().end() <= h.text_range().start())
                && cx.tag(&child, "statement");
            let is = |n: &SyntaxNode| cx.tag(n, "reference") && code_text(n) == guard;
            if in_body
                && handlers.iter().any(|h| {
                    cx.slot(h, "type").is_some_and(|t| {
                        is(&t) || (cx.tag(&t, "collection") && t.children().any(|e| is(&e)))
                    })
                })
            {
                return true;
            }
        }
        child = a;
    }
    false
}

/// Where the first binding of a symbol takes effect; `None` when `global` or `nonlocal` binds the
/// name for the whole scope.
fn first_effect(cx: &Cx, bs: &Bindings<'_>, id: SymbolId) -> Option<TextSize> {
    let sym = cx.m.symbol(id);
    if sym
        .decls
        .iter()
        .any(|d| !own(cx, sym.scope, d) || d.flags.contains(Flags::REDIRECT))
    {
        return None;
    }
    Some(bs.get(id).first()?.effect)
}

/// The reads of names that nothing binds: undefined names, and the names a wildcard import may
/// bring. A read that runs before the first binding of its name in the scope finds the name
/// outside.
pub(super) fn unresolved(cx: &Cx, bs: &Bindings<'_>) -> (Vec<Hit>, Vec<Hit>) {
    let (m, lang) = (cx.m, cx.lang);
    let (mut undefined, mut starred) = (Vec::new(), Vec::new());
    let mut firsts: HashMap<SymbolId, Option<TextSize>> = HashMap::new();
    let class = |s: ScopeId| cx.tag(&m.scope(s).node, "type-declaration");
    for r in m.refs.iter().filter(|r| r.is_read()) {
        let name = &*r.name;
        let sym = r.symbol.map(|s| m.symbol(s));
        // `del x` unbinds a name of its own scope only.
        if r.is_delete() {
            if m.own(r.scope, name).is_none() && m.redirect(r.scope, name).is_none() {
                undefined.push((r.range, name.to_owned()));
            }
            continue;
        }
        // A name that has only `x: int` is not bound.
        let bare = sym.is_some_and(|s| {
            !class(s.scope) && s.decls.iter().all(|d| d.flags.contains(Flags::BARE))
        });
        if bare {
            undefined.push((r.range, name.to_owned()));
            continue;
        }
        let before = match (r.symbol, sym) {
            (Some(id), Some(s)) if !r.is_type() && !deferred(cx, r.scope, s.scope) => {
                let first = *firsts.entry(id).or_insert_with(|| first_effect(cx, bs, id));
                first.is_some_and(|e| r.range.start() < e)
                    && !m
                        .ancestors(s.scope)
                        .skip(1)
                        .any(|o| !m.scope(o).hidden && m.own(o, name).is_some())
            }
            _ => false,
        };
        if (sym.is_some() && !before)
            || lang.is_global(name, cx.file.path)
            || m.ancestors(r.scope)
                .any(|s| lang.scope_name(cx.file.grammar.name(m.scope(s).node.kind()), name))
        {
            continue;
        }
        if m.ancestors(r.scope).any(|s| m.own(s, "*").is_some()) {
            starred.push((r.range, name.to_owned()));
        } else if cx.ordered().guard.is_empty() || !guarded(cx, &cx.node_of(r)) {
            undefined.push((r.range, name.to_owned()));
        }
    }
    (undefined, starred)
}

/// The exported names no scope binds, and the exported names a wildcard import may bring.
pub(super) fn export_findings(
    cx: &Cx,
    exported: &[(String, TextRange)],
    star_usage: &mut Vec<Hit>,
) -> Vec<Hit> {
    let stars = cx.m.own(ScopeId(0), "*").is_some();
    for (name, range) in exported {
        if stars && cx.m.own(ScopeId(0), name).is_none() {
            star_usage.push((*range, name.clone()));
        }
    }
    if cx.lang.flavor("init", cx.file.path) || stars {
        return Vec::new();
    }
    (exported.iter())
        .filter(|(name, _)| cx.m.own(ScopeId(0), name).is_none())
        .map(|(name, range)| (*range, name.clone()))
        .collect()
}

/// A name read in a function before the function assigns it, while an enclosing scope binds it
/// too.
pub(super) fn use_before_define(cx: &Cx, bs: &Bindings<'_>) -> Vec<Hit> {
    let m = cx.m;
    let def = |s: ScopeId| {
        let n = &m.scope(s).node;
        cx.is_function(n) && cx.tag(n, "statement")
    };
    let mut hits = Vec::new();
    for id in m.symbol_ids() {
        let sym = m.symbol(id);
        if !def(sym.scope) || sym.decls.first().is_none_or(|d| !own(cx, sym.scope, d)) {
            continue;
        }
        let b = &bs.get(id)[0];
        if matches!(
            b.kind,
            Kind::Annotation
                | Kind::Argument
                | Kind::Except
                | Kind::Global
                | Kind::Import(Form::Pragma)
        ) {
            continue;
        }
        let read = m
            .refs_of(id)
            .find(|r| r.is_read() && r.scope == sym.scope && r.range.start() < b.effect);
        let Some(read) = read else { continue };
        let outer = m.ancestors(sym.scope).skip(1).find_map(|s| {
            (def(s) || m.scope(s).parent.is_none())
                .then(|| m.own(s, &sym.name))
                .flatten()
        });
        if outer.is_some() {
            hits.push((read.range, sym.name.to_string()));
        }
    }
    hits
}
