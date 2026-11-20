//! A mapping of keys that each list values, with entries that add (`include`) and remove
//! (`exclude`) combinations (a job matrix): a value listed twice, an exclusion of a key or value
//! that no combination has. The language gives the two key names; the checks are here.
//!
//! concept: none (infrastructure: shape typed entries)

use super::{Tree, Val, Walk};
use crate::shape::Row;
use core_syntax::{SyntaxNode, TextRange};

pub struct Combos {
    pub include: &'static str,
    pub exclude: &'static str,
}

type Entries = [(String, TextRange, Option<SyntaxNode>)];

/// Whether the excluded value `ex` names (is part of) the listed value `listed`.
fn matches(ex: &Tree, listed: &Tree, marker: &str) -> bool {
    match (ex, listed) {
        (Tree::Scalar(a, _), Tree::Scalar(b, _)) => {
            a == b || a.contains(marker) || b.contains(marker)
        }
        (Tree::Scalar(a, _), _) | (_, Tree::Scalar(a, _)) if a.contains(marker) => true,
        (Tree::Map(a), Tree::Map(b)) => a
            .iter()
            .all(|(k, v)| b.iter().any(|(k2, v2)| k == k2 && matches(v, v2, marker))),
        (Tree::Seq(a), Tree::Seq(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(x, y)| matches(x, y, marker))
        }
        _ => false,
    }
}

/// The value written as JSON, as the messages quote it.
fn show(tree: &Tree) -> String {
    match tree {
        Tree::Null => "null".into(),
        Tree::Scalar(s, _) => format!("\"{s}\""),
        Tree::Seq(items) => format!(
            "[{}]",
            items.iter().map(show).collect::<Vec<_>>().join(", ")
        ),
        Tree::Map(m) => format!(
            "{{{}}}",
            m.iter()
                .map(|(k, v)| format!("\"{k}\": {}", show(v)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn same(a: &Tree, b: &Tree) -> bool {
    match (a, b) {
        (Tree::Scalar(x, _), Tree::Scalar(y, _)) => x == y,
        (Tree::Map(x), Tree::Map(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.iter().any(|(k2, v2)| k == k2 && same(v, v2)))
        }
        (Tree::Seq(x), Tree::Seq(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same(p, q))
        }
        (Tree::Null, Tree::Null) => true,
        _ => false,
    }
}

impl Walk<'_> {
    fn items_of(&self, node: &Option<SyntaxNode>) -> Vec<(TextRange, Tree)> {
        match self.val(node.as_ref()) {
            Val::Seq(seq) => (self.items(&seq).into_iter())
                .map(|(range, value)| (range, self.tree(value)))
                .collect(),
            _ => Vec::new(),
        }
    }

    pub(super) fn combos(&mut self, row: &'static Row, c: &Combos, entries: &Entries) {
        let marker = self.shape.expression;
        let opens =
            |t: &Tree| matches!(t, Tree::Scalar(s, _) if !marker.is_empty() && s.contains(marker));
        // Each key with the values it lists, and whether they are only known at run time.
        let mut keys: Vec<(String, Vec<Tree>, bool)> = Vec::new();
        for (name, _, value) in entries {
            if name == c.include || name == c.exclude {
                continue;
            }
            let items = self.items_of(value);
            let open = !matches!(self.val(value.as_ref()), Val::Seq(_))
                || items.iter().any(|(_, t)| opens(t));
            for (i, (range, t)) in items.iter().enumerate() {
                if items[..i].iter().any(|(_, u)| same(t, u)) {
                    self.report(
                        row,
                        *range,
                        format!("duplicate value {} is found in matrix \"{name}\"", show(t)),
                    );
                }
            }
            let listed = items.into_iter().map(|(_, t)| t).collect();
            keys.push((name.clone(), listed, open));
        }
        for (name, _, value) in entries {
            if name != c.include {
                continue;
            }
            for (_, item) in self.items_of(value) {
                let Tree::Map(m) = item else { continue };
                for (k, v) in m {
                    match keys.iter_mut().find(|e| e.0 == k) {
                        Some(e) => e.1.push(v),
                        None => keys.push((k, vec![v], false)),
                    }
                }
            }
        }
        for (name, _, value) in entries {
            if name != c.exclude {
                continue;
            }
            let Val::Seq(seq) = self.val(value.as_ref()) else {
                continue;
            };
            for (_, item) in self.items(&seq) {
                let Val::Map(map) = self.val(item.as_ref()) else {
                    continue;
                };
                for (key, range, v) in self.entries(&map) {
                    let Some((_, listed, open)) = keys.iter().find(|e| e.0 == key) else {
                        let message = format!(
                            "\"{key}\" in \"{}\" section does not exist in matrix",
                            c.exclude
                        );
                        self.report(row, range, message);
                        continue;
                    };
                    let at = v
                        .as_ref()
                        .map_or(range, |n| crate::tree::significant_range(n, self.g));
                    let ex = self.tree(v);
                    if !*open && !listed.iter().any(|l| matches(&ex, l, marker)) {
                        let message = format!(
                            "value {} in \"{}\" does not match in matrix \"{key}\" combinations",
                            show(&ex),
                            c.exclude
                        );
                        self.report(row, at, message);
                    }
                }
            }
        }
    }
}
