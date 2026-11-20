//! Contexts whose type comes from the file itself: the step ids before a step, the jobs a job
//! needs, the keys of a matrix, the inputs of a workflow. The language lists the facts (key
//! paths, key names, the type text of one entry); the lookup is here.
//!
//! concept: none (infrastructure: expression language)

use super::ty::Ty;
use crate::shape::Tree;

#[derive(Clone, Copy, PartialEq)]
pub enum ScopeKind {
    /// Names are the `key` values of the items of the sequence `source` in the scope's mapping;
    /// inside that sequence only the items before the current one count.
    Ids,
    /// Names are the jobs listed (a name or a list) at `source`; each is `item` plus the property
    /// `output` holding the keys of that job's `output` mapping.
    Refs,
    /// Names are the keys of the mapping at `source` (`a|b`: any of them), except `skip`, plus the
    /// keys of the items of the sequence `also`; the entry's type is `item`, or the one that
    /// `types` gives for the entry's `key` value; `infer` as `item`: the type of the entry's values.
    Keys,
}

pub struct Scope {
    pub context: &'static str,
    pub kind: ScopeKind,
    /// The key path (`*` is one key) whose node the `source` paths are relative to; empty: the file.
    pub within: &'static str,
    pub source: &'static str,
    pub key: &'static str,
    pub item: &'static str,
    pub output: &'static str,
    /// For `Refs`: the key of the file that holds the jobs.
    pub from: &'static str,
    pub skip: &'static [&'static str],
    pub also: &'static str,
    /// For `Keys`: names that exist besides the ones in the file (of type `item`).
    pub base: &'static [&'static str],
    pub types: &'static [(&'static str, &'static str)],
    /// For `Keys`: the context is an empty object when the file has no mapping at `source`.
    pub strict: bool,
    /// With `strict`: only when the file has a node at this key path (empty: always).
    pub only_with: &'static str,
}

impl Scope {
    pub const NONE: Scope = Scope {
        context: "",
        kind: ScopeKind::Keys,
        within: "",
        source: "",
        key: "",
        item: "any",
        output: "",
        from: "",
        skip: &[],
        also: "",
        base: &[],
        types: &[],
        strict: false,
        only_with: "",
    };
}

fn parts(path: &str) -> Vec<&str> {
    path.split('.').filter(|s| !s.is_empty()).collect()
}

fn names(tree: &Tree) -> Vec<String> {
    match tree {
        Tree::Scalar(s, _) => vec![s.clone()],
        Tree::Seq(items) => items
            .iter()
            .filter_map(|i| match i {
                Tree::Scalar(s, _) => Some(s.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn props(item: &str) -> Vec<(String, Ty)> {
    match Ty::parse(item) {
        Ty::Obj(p) => p,
        _ => Vec::new(),
    }
}

/// The type of the scope's context for an expression at `path`; `None`: not known (any).
pub fn ty(
    scope: &Scope,
    root: &Tree,
    path: &[String],
    eval: &dyn Fn(&str) -> Option<Ty>,
    marker: &str,
) -> Option<Ty> {
    let n = parts(scope.within).len();
    if path.len() < n {
        return None;
    }
    let base = root.at(&path[..n])?;
    match scope.kind {
        ScopeKind::Ids => {
            let seq = parts(scope.source);
            let inside = path
                .get(n..n + seq.len())
                .is_some_and(|p| p == seq.as_slice());
            let limit = if inside {
                path.get(n + seq.len())
                    .and_then(|i| i.parse::<usize>().ok())
            } else {
                None
            };
            let Some(Tree::Seq(items)) = base.at(&seq) else {
                return Some(Ty::Obj(Vec::new()));
            };
            let item = Ty::parse(scope.item);
            let found = items.iter().take(limit.unwrap_or(items.len()));
            let ids = found.filter_map(|i| match i.get(scope.key) {
                Some(Tree::Scalar(id, _)) => Some((id.clone(), item.clone())),
                _ => None,
            });
            Some(Ty::Obj(ids.collect()))
        }
        ScopeKind::Refs => {
            let jobs = root.get(scope.from)?;
            let all: Vec<String> = match jobs {
                Tree::Map(m) if scope.source.is_empty() => {
                    m.iter().map(|(k, _)| k.clone()).collect()
                }
                _ => Vec::new(),
            };
            let listed = if scope.source.is_empty() {
                all
            } else {
                let Some(listed) = base.at(&parts(scope.source)) else {
                    return Some(Ty::Obj(Vec::new()));
                };
                names(listed)
            };
            let mut out = Vec::new();
            for name in listed {
                let mut p = props(scope.item);
                let outputs = match jobs.get(&name).and_then(|j| j.get(scope.output)) {
                    Some(Tree::Map(m)) => {
                        Ty::Obj(m.iter().map(|(k, _)| (k.clone(), Ty::Str)).collect())
                    }
                    _ if jobs.get(&name).is_some() => Ty::Obj(Vec::new()),
                    _ => Ty::Map(Box::new(Ty::Str)),
                };
                p.push((scope.output.to_string(), outputs));
                out.push((name, Ty::Obj(p)));
            }
            Some(Ty::Obj(out))
        }
        ScopeKind::Keys => {
            let mut found = false;
            let mut out: Vec<(String, Ty)> = Vec::new();
            for alt in scope.source.split('|') {
                let seq = parts(alt);
                let inside = path
                    .get(n..n + seq.len())
                    .is_some_and(|p| p == seq.as_slice());
                let before = if inside {
                    path.get(n + seq.len())
                } else {
                    None
                };
                match base.at(&seq) {
                    Some(Tree::Map(map)) => {
                        found = true;
                        for (k, v) in map {
                            if before == Some(k) {
                                break;
                            }
                            if scope.skip.contains(&k.as_str()) {
                                continue;
                            }
                            let mapped = match v.get(scope.key) {
                                Some(Tree::Scalar(t, _)) => {
                                    scope.types.iter().find(|(a, _)| a == t)
                                }
                                _ => None,
                            };
                            let t = match (mapped, scope.item) {
                                (Some(m), _) => Ty::parse(m.1),
                                (None, "infer") => match v {
                                    Tree::Seq(items) => {
                                        unify(items.iter().map(|i| infer(i, eval, marker)))
                                    }
                                    other => infer(other, eval, marker),
                                },
                                (None, text) => Ty::parse(text),
                            };
                            out.push((k.clone(), t));
                        }
                        if let Some(Tree::Seq(items)) =
                            map.iter().find(|(k, _)| k == scope.also).map(|m| &m.1)
                        {
                            for item in items {
                                if let Tree::Map(m) = item {
                                    out.extend(
                                        m.iter().map(|(k, v)| (k.clone(), infer(v, eval, marker))),
                                    );
                                }
                            }
                        }
                    }
                    Some(Tree::Scalar(text, _)) if !text.contains(marker) => found = true,
                    Some(Tree::Null) => found = true,
                    Some(_) => return None,
                    None => {}
                }
            }
            out.extend(
                scope
                    .base
                    .iter()
                    .map(|k| (k.to_string(), Ty::parse(scope.item))),
            );
            (found
                || scope.strict
                    && (scope.only_with.is_empty() || root.at(&parts(scope.only_with)).is_some()))
            .then_some(Ty::Obj(out))
        }
    }
}

/// The type of a value written in the file.
fn infer(tree: &Tree, eval: &dyn Fn(&str) -> Option<Ty>, marker: &str) -> Ty {
    match tree {
        Tree::Null => Ty::Null,
        Tree::Scalar(t, false) if t == "true" || t == "false" => Ty::Bool,
        Tree::Scalar(t, false) if t.parse::<f64>().is_ok() => Ty::Num,
        Tree::Scalar(t, _) if t.contains(marker) => eval(t).unwrap_or(Ty::Any),
        Tree::Scalar(..) => Ty::Str,
        Tree::Seq(items) => Ty::Arr(Box::new(unify(
            items.iter().map(|i| infer(i, eval, marker)),
        ))),
        Tree::Map(m) => Ty::Obj(
            m.iter()
                .map(|(k, v)| (k.clone(), infer(v, eval, marker)))
                .collect(),
        ),
    }
}

/// The one type all of `types` share, or any.
fn unify(types: impl Iterator<Item = Ty>) -> Ty {
    let mut all = types;
    let Some(first) = all.next() else {
        return Ty::Any;
    };
    if all.all(|t| t == first) {
        first
    } else {
        Ty::Any
    }
}
