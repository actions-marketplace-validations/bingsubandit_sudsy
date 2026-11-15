//! What only the whole file shows: which imports and variables nothing read, which names of
//! `__all__` do not exist, which imports a function's own names cover.

use super::model::{Binding, Kind, Model, Scope, ScopeKind, is_dummy};

pub fn finish(m: &mut Model<'_>, init: bool) {
    exports(m, init);
    m.report_shadows();
    for sid in 0..m.scopes.len() {
        let mut found: Vec<(&'static str, ruff_text_size::TextRange, String)> = Vec::new();
        let scope = &m.scopes[sid];
        for &bid in scope.bindings.values() {
            let b = &m.bindings[bid];
            if b.used {
                continue;
            }
            match b.kind {
                Kind::Import | Kind::SubmoduleImport | Kind::FromImport
                    if !b.explicit_export
                        && scope.kind != ScopeKind::Class
                        && !(b.kind == Kind::SubmoduleImport && alias_used(m, scope, b)) =>
                {
                    found.push(("unused-import", b.range, b.label.to_string()));
                }
                Kind::Assignment | Kind::NamedExpr | Kind::WithItem
                    if scope.kind.is_function()
                        && !scope.uses_locals
                        && !b.unpacked
                        && !unused_ok(b.name) =>
                {
                    found.push(("unused-variable", b.range, b.name.to_string()));
                }
                Kind::Annotation
                    if scope.kind.is_function() && !scope.uses_locals && !unused_ok(b.name) =>
                {
                    found.push(("unused-annotation", b.range, b.name.to_string()));
                }
                _ => {}
            }
        }
        for (check, range, name) in found {
            m.hit(check, range, name);
        }
    }
}

/// `import a.b` next to a used `import a as x`: reading `x` reaches `a.b` too.
fn alias_used(m: &Model<'_>, scope: &Scope<'_>, sub: &Binding<'_>) -> bool {
    scope.bindings.values().any(|&id| {
        let o = &m.bindings[id];
        o.kind == Kind::Import && o.used && o.target == sub.target
    })
}

fn unused_ok(name: &str) -> bool {
    is_dummy(name)
        || matches!(
            name,
            "__tracebackhide__" | "__traceback_info__" | "__traceback_supplement__"
        )
}

/// Marks what `__all__` names as read and reports the names that are not there (not in an
/// `__init__.py`, whose `__all__` may list submodules).
fn exports(m: &mut Model<'_>, init: bool) {
    let exports = std::mem::take(&mut m.scopes[0].exports);
    let stars = !m.scopes[0].stars.is_empty();
    for (name, range) in exports {
        match m.scopes[0].bindings.get(name.as_str()) {
            Some(&b) => m.bindings[b].used = true,
            None if init && !stars => {}
            None if !stars => m.hit("undefined-export", range, name),
            None => m.hit("undefined-local-with-import-star-usage", range, name),
        }
    }
}
