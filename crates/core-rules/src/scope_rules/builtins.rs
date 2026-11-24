//! `redefines-builtin`: a declaration named like an identifier the language predeclares (the
//! `globals` of the language: Go `len`, `string`, `nil`; Python `list`, `id`). Options: `allow`
//! (names that may be declared), `kinds` (what is checked, as in `shadowed-variable`; empty: all)
//! `except_parents` (grammar kinds whose direct declarations are left out: a Go receiver) and
//! `assigned_by` (grammar kinds of statements whose left side names are checked too: a Go
//! assignment `len = 1`).
//!
//! concept: redefines-builtin
//! what: A declaration whose name is one the language predeclares.
//! rules: redefines-builtin-id
//! reads: rule:redefines-builtin-id

use super::cx::Cx;
use super::shadowed::of_kind;
use core_syntax::TextRange;

pub(super) struct Opts<'a> {
    pub allow: Vec<&'a str>,
    pub kinds: Vec<&'a str>,
    pub except_parents: Vec<&'a str>,
    pub assigned_by: Vec<&'a str>,
}

pub(super) fn run(cx: &Cx, o: &Opts) -> Vec<(TextRange, String)> {
    let mut hits = Vec::new();
    for id in cx.m.symbol_ids() {
        let s = cx.m.symbol(id);
        if o.allow.contains(&&*s.name) || !cx.lang.is_global(&s.name, cx.file.path) {
            continue;
        }
        for d in &s.decls {
            let left_out = cx
                .decl_node(d)
                .parent()
                .is_some_and(|p| o.except_parents.contains(&cx.file.grammar.name(p.kind())));
            if of_kind(d.flags, &o.kinds) && !left_out {
                hits.push((d.name, s.name.to_string()));
            }
        }
    }
    if !o.assigned_by.is_empty() {
        let names: std::collections::HashSet<_> = cx.m.refs.iter().map(|r| r.range).collect();
        for n in cx.file.root.descendants() {
            if !o.assigned_by.contains(&cx.file.grammar.name(n.kind())) {
                continue;
            }
            let Some(left) = n.first_child() else {
                continue;
            };
            for t in left.children_with_tokens().filter_map(|e| e.into_token()) {
                if names.contains(&t.text_range()) && cx.lang.is_global(t.text(), cx.file.path) {
                    hits.push((t.text_range(), t.text().to_string()));
                }
            }
        }
    }
    hits.sort_by_key(|(r, _)| r.start());
    hits.dedup();
    hits
}
