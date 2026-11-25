//! The code a preprocessor directive carries (the parameter list and the body of a define, the
//! condition of an if), found in the tokens of the directives and read again as a small file of its
//! own with the layers of the language. A payload whose brackets do not pair stays text.
//!
//! concept: none (glue: nested parse of directive payloads)

use crate::adapter::Node;
use core_syntax::SyntaxKind;

/// A token of a lexer that knows directives.
#[derive(Clone, Copy)]
pub struct DirectiveToken {
    pub kind: SyntaxKind,
    pub start: u32,
    pub end: u32,
    /// The directive the token belongs to, counted from 1; 0 outside directives.
    pub dir: u32,
}

/// The facts of a language's directives.
pub struct Directives<'a> {
    /// The words of the directives whose rest is an expression.
    pub conditions: &'a [&'a str],
    /// The word of the directive that defines a name, and the kind of its parameter group.
    pub define: &'a str,
    pub params: SyntaxKind,
    /// The bracket pairs (opener, closer); the first is the one of the parameter group.
    pub brackets: &'a [(SyntaxKind, SyntaxKind)],
    /// Reads tokens as a body (`false`) or as an expression (`true`).
    pub build: &'a dyn Fn(&[DirectiveToken], bool) -> Vec<Node>,
}

/// The nodes of the payloads of all directives, in opening order.
pub fn payload_nodes(src: &str, toks: &[DirectiveToken], d: &Directives) -> Vec<Node> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < toks.len() {
        if toks[i].dir == 0 {
            i += 1;
            continue;
        }
        let mut j = i;
        while j + 1 < toks.len() && toks[j + 1].dir == toks[i].dir {
            j += 1;
        }
        directive(src, &toks[i..=j], d, &mut out);
        i = j + 1;
    }
    out
}

fn directive(src: &str, toks: &[DirectiveToken], d: &Directives, out: &mut Vec<Node>) {
    let word = |i: usize| toks.get(i).map(|t| &src[t.start as usize..t.end as usize]);
    let condition = word(1).is_some_and(|w| d.conditions.contains(&w));
    let mut params = false;
    let from = if condition {
        2
    } else if word(1) == Some(d.define) && toks.len() > 2 {
        // (`#define F(a, b)`: the `(` touches the name)
        match toks.get(3) {
            Some(l) if l.kind == d.brackets[0].0 && l.start == toks[2].end => {
                let Some(close) = bracket_end(toks, d.brackets, 3) else {
                    return;
                };
                out.push(Node {
                    start: l.start,
                    end: toks[close].end,
                    kind: d.params,
                });
                params = true;
                close + 1
            }
            _ => 3,
        }
    } else {
        return;
    };
    let body: Vec<DirectiveToken> = toks[from.min(toks.len())..]
        .iter()
        .map(|t| DirectiveToken { dir: 0, ..*t })
        .collect();
    // (a body of one token, or none, has no layout of its own)
    if (body.len() > 1 || params) && brackets_pair(&body, d.brackets) {
        out.extend((d.build)(&body, condition));
    }
}

/// The nodes of `extra` merged into `nodes` (both in opening order); an extra node starts where no
/// other node starts.
pub fn merge_nodes(nodes: Vec<Node>, extra: Vec<Node>) -> Vec<Node> {
    if extra.is_empty() {
        return nodes;
    }
    let mut out = Vec::with_capacity(nodes.len() + extra.len());
    let mut extra = extra.into_iter().peekable();
    for node in nodes {
        while let Some(p) = extra.next_if(|p| p.start < node.start) {
            out.push(p);
        }
        out.push(node);
    }
    out.extend(extra);
    out
}

/// Where the bracket of `toks[open]` closes, scanning from `open` with the bracket `pairs`
/// (opener, closer): `None` when a bracket meets the wrong partner or none.
fn bracket_end(
    toks: &[DirectiveToken],
    pairs: &[(SyntaxKind, SyntaxKind)],
    open: usize,
) -> Option<usize> {
    let mut stack = Vec::new();
    for (i, t) in toks.iter().enumerate().skip(open) {
        if let Some(&(_, close)) = pairs.iter().find(|p| p.0 == t.kind) {
            stack.push(close);
        } else if pairs.iter().any(|p| p.1 == t.kind) {
            if stack.pop() != Some(t.kind) {
                return None;
            }
            if stack.is_empty() {
                return Some(i);
            }
        }
    }
    None
}

/// Whether every bracket of `toks` meets its partner.
fn brackets_pair(toks: &[DirectiveToken], pairs: &[(SyntaxKind, SyntaxKind)]) -> bool {
    let mut stack = Vec::new();
    toks.iter().all(|t| {
        if let Some(&(_, close)) = pairs.iter().find(|p| p.0 == t.kind) {
            stack.push(close);
            true
        } else if pairs.iter().any(|p| p.1 == t.kind) {
            stack.pop() == Some(t.kind)
        } else {
            true
        }
    }) && stack.is_empty()
}
