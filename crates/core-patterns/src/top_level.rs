//! The atoms that recognise a document by content: `top-level` (the plain scalar entries of the
//! top-level mapping match regexes; named groups are captured at the value) and
//! `top-level-in-option` (the values of some of those entries, joined by `/`, are an item of a
//! list option: a table of the kinds of document a tool knows).
//!
//! concept: pattern-rules
//! what: ast-grep style pattern rules: atoms, relations and combinators on the shared tree.
//! tools: ast-grep rules
//! reads: rulefiles:pattern

use crate::expr::Ctx;
use crate::matcher::{Capture, Captures};
use core_rules::{Re, Scalar};
use core_syntax::{Grammar, SyntaxElement, SyntaxNode};
use serde::Deserialize;

/// The `top-level-in-option` atom as written.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InOption {
    /// The top-level keys whose values are joined with `/`.
    pub keys: Vec<String>,
    /// The `list` option that holds the joined values.
    pub option: String,
}

/// Is `node` the mapping at the top of its document?
fn is_top(node: &SyntaxNode, g: &Grammar) -> bool {
    let tagged = |n: &SyntaxNode| g.has_tag(n.kind(), "mapping") || g.has_tag(n.kind(), "sequence");
    tagged(node) && !std::iter::successors(node.parent(), SyntaxNode::parent).any(|a| tagged(&a))
}

/// The plain string value of the entry `key` of the top-level mapping `node`, with its node.
fn value<'a>(node: &SyntaxNode, key: &str, cx: &Ctx<'a>) -> Option<(String, SyntaxNode)> {
    let g = cx.grammar;
    node.children()
        .filter(|e| g.has_tag(e.kind(), "key-value"))
        .find(|e| {
            g.slot(e, "key")
                .is_some_and(|k| cx.hooks.key_name(g, &k, cx.text) == key)
        })
        .and_then(|e| {
            let SyntaxElement::Node(v) = g.slot(&e, "value")? else {
                return None;
            };
            match cx.hooks.scalar(g, &v, cx.text)? {
                Scalar::String(s) => Some((s.into_owned(), v)),
                Scalar::Number(_) => None,
            }
        })
}

/// `top-level`: every entry has a value that matches its regex.
pub fn matches(
    entries: &[(String, Re, Vec<String>)],
    node: &SyntaxNode,
    cx: &Ctx<'_>,
) -> Option<Captures> {
    if !is_top(node, cx.grammar) {
        return None;
    }
    let mut out = Captures::new();
    for (key, re, groups) in entries {
        let (text, v) = value(node, key, cx)?;
        if groups.is_empty() {
            re.is_match(&text).then_some(())?;
            continue;
        }
        let spans = re.groups_in(&text, false)?;
        let range = cx.ranges.significant_range(&v, cx.grammar);
        // A plain scalar is its source text, so a group has its own span; else the whole value.
        let exact = cx.text.get(range.start().into()..range.end().into()) == Some(text.as_str());
        out.extend(
            Capture::groups(groups, spans, range.start()).map(|(name, c)| {
                if exact {
                    (name, c)
                } else {
                    (name, Capture::span(range))
                }
            }),
        );
    }
    Some(out)
}

/// `top-level-in-option`.
pub fn in_option(spec: &InOption, node: &SyntaxNode, cx: &Ctx<'_>) -> Option<Captures> {
    if !is_top(node, cx.grammar) {
        return None;
    }
    let mut joined = String::new();
    for (i, key) in spec.keys.iter().enumerate() {
        if i > 0 {
            joined.push('/');
        }
        joined.push_str(&value(node, key, cx)?.0);
    }
    cx.options
        .list_has(&spec.option, &joined, false)
        .then(Captures::new)
}
