//! Token spacing as data: a language's rows say which blanks stand between two neighbouring
//! tokens (`Flow::want` and its companions), so a spacing change is a row, not a function.
//!
//! The text has sections `[space]`, `[joined]`, `[at_least_one]` and `[op_first]`; a line is
//! `PREV / NEXT : RESULT` (`[op_first]`: one pattern). The first row of a section that matches
//! wins. `RESULT` is `yes` (one blank), `no` or `keep` (as the source has it); the other sections
//! have no result. A pattern is `*` or conditions joined by `&`: `key=a|b` or `key!=a|b` with
//! `key` one of `kind` and `parent` and `grand` (names of the grammar's kinds, of the token, of
//! its parent node and of that node's parent), `end_of` (the token ends a node of one of the kinds)
//! and `text`, `sibling` and `before` (texts, case-insensitive: of the token, of any other token
//! of its parent node, of any token of the parent node before it). Lines starting with `#` are
//! comments. [`Pattern`] is also the matcher of `rewrite::RewriteRules`.

use core_syntax::{Grammar, SyntaxElement, SyntaxKind, SyntaxToken};

/// A condition on a token: the token's value is (or, with `not`, is not) one of `any`.
struct Cond {
    key: Key,
    not: bool,
    kinds: Vec<SyntaxKind>,
    texts: Vec<String>,
}

#[derive(Clone, Copy)]
enum Key {
    Kind,
    Parent,
    Grand,
    EndOf,
    Text,
    Sibling,
    Before,
}

#[derive(Default)]
pub struct Pattern(Vec<Cond>);

impl Pattern {
    pub fn parse(grammar: &Grammar, text: &str) -> Pattern {
        let conds = text.trim().split('&').filter(|c| *c != "*").map(|c| {
            let (key, not, vals) = match c.split_once("!=") {
                Some((k, v)) => (k, true, v),
                None => {
                    let (k, v) = c
                        .split_once('=')
                        .unwrap_or_else(|| panic!("spacing: `{c}`"));
                    (k, false, v)
                }
            };
            let key = match key.trim() {
                "kind" => Key::Kind,
                "parent" => Key::Parent,
                "grand" => Key::Grand,
                "end_of" => Key::EndOf,
                "text" => Key::Text,
                "sibling" => Key::Sibling,
                "before" => Key::Before,
                other => panic!("spacing: unknown key `{other}`"),
            };
            let vals = vals.split('|').map(str::trim);
            let (mut kinds, mut texts) = (Vec::new(), Vec::new());
            for v in vals {
                if matches!(key, Key::Text | Key::Sibling | Key::Before) {
                    texts.push(v.to_ascii_lowercase());
                } else {
                    kinds.push(
                        grammar
                            .kind_by_name(v)
                            .unwrap_or_else(|| panic!("spacing: unknown kind `{v}`")),
                    );
                }
            }
            Cond {
                key,
                not,
                kinds,
                texts,
            }
        });
        Pattern(conds.collect())
    }

    pub fn matches(&self, t: &SyntaxToken) -> bool {
        self.0.iter().all(|c| {
            let hit = match c.key {
                Key::Kind => c.kinds.contains(&t.kind()),
                Key::Text => c.texts.iter().any(|x| t.text().eq_ignore_ascii_case(x)),
                Key::Sibling | Key::Before => {
                    let before = matches!(c.key, Key::Before);
                    let mut found = false;
                    let mut cur = if before {
                        t.prev_sibling_or_token()
                    } else {
                        t.parent().and_then(|p| p.first_child_or_token())
                    };
                    while let Some(e) = cur {
                        if let SyntaxElement::Token(o) = &e
                            && o != t
                            && c.texts.iter().any(|x| o.text().eq_ignore_ascii_case(x))
                        {
                            found = true;
                            break;
                        }
                        cur = if before {
                            e.prev_sibling_or_token()
                        } else {
                            e.next_sibling_or_token()
                        };
                    }
                    found
                }
                Key::EndOf => t.parent().is_some_and(|mut n| {
                    loop {
                        if last_code(&n).is_none_or(|l| l != *t) {
                            return false;
                        }
                        if c.kinds.contains(&n.kind()) {
                            return true;
                        }
                        match n.parent() {
                            Some(p) => n = p,
                            None => return false,
                        }
                    }
                }),
                Key::Parent => t.parent().is_some_and(|p| c.kinds.contains(&p.kind())),
                Key::Grand => t
                    .parent()
                    .and_then(|p| p.parent())
                    .is_some_and(|p| c.kinds.contains(&p.kind())),
            };
            hit != c.not
        })
    }
}

/// The last token of the node that is not blank.
fn last_code(n: &core_syntax::SyntaxNode) -> Option<SyntaxToken> {
    let mut cur = n.last_token();
    while let Some(t) = &cur {
        if !t.text().trim().is_empty() {
            break;
        }
        cur = t.prev_token();
    }
    cur
}

struct Row {
    prev: Pattern,
    next: Pattern,
    result: Option<bool>,
}

impl Row {
    fn matches(&self, a: &SyntaxToken, b: &SyntaxToken) -> bool {
        self.prev.matches(a) && self.next.matches(b)
    }
}

/// The rows of one language, compiled against its grammar.
pub struct SpaceRules {
    space: Vec<Row>,
    joined: Vec<Row>,
    at_least_one: Vec<Row>,
    op_first: Vec<Pattern>,
    flat_after: Vec<SyntaxKind>,
    closers: Vec<SyntaxKind>,
    unhang: Vec<SyntaxKind>,
}

fn row(grammar: &Grammar, line: &str) -> Row {
    let (pair, result) = line.split_once(':').unwrap_or((line, ""));
    let (prev, next) = pair
        .split_once('/')
        .unwrap_or_else(|| panic!("spacing: `{line}`"));
    Row {
        prev: Pattern::parse(grammar, prev),
        next: Pattern::parse(grammar, next),
        result: match result.trim() {
            "yes" => Some(true),
            "no" => Some(false),
            _ => None,
        },
    }
}

impl SpaceRules {
    /// The sections `[space]`, `[joined]`, `[at_least_one]`, `[op_first]` and the kind lists
    /// `flat_after`, `closers` and `unhang` of `[flow]`.
    pub fn parse(grammar: &Grammar, text: &str) -> SpaceRules {
        let rows = |name| {
            crate::rules::section(text, name)
                .map(|l| row(grammar, l))
                .collect()
        };
        let flow = crate::rules::settings(text, "flow");
        let kinds = |key: &str| -> Vec<SyntaxKind> {
            flow.iter()
                .filter(|(k, _)| *k == key)
                .flat_map(|(_, v)| v.split('|'))
                .map(|n| {
                    grammar
                        .kind_by_name(n.trim())
                        .unwrap_or_else(|| panic!("spacing: unknown kind `{n}`"))
                })
                .collect()
        };
        SpaceRules {
            space: rows("space"),
            joined: rows("joined"),
            at_least_one: rows("at_least_one"),
            op_first: crate::rules::section(text, "op_first")
                .map(|l| Pattern::parse(grammar, l))
                .collect(),
            flat_after: kinds("flat_after"),
            closers: kinds("closers"),
            unhang: kinds("unhang"),
        }
    }

    /// `Flow::flat_after`: a line break after this element starts no hanging part.
    pub fn flat_after(&self, kind: SyntaxKind) -> bool {
        self.flat_after.contains(&kind)
    }

    /// `Flow::closers`.
    pub fn closer(&self, kind: SyntaxKind) -> bool {
        self.closers.contains(&kind)
    }

    /// `Flow::unhang`.
    pub fn unhangs(&self, kind: SyntaxKind) -> bool {
        self.unhang.contains(&kind)
    }

    /// `Flow::want`: a blank between the tokens (`None`: as the source has it).
    pub fn want(&self, a: &SyntaxToken, b: &SyntaxToken) -> Option<bool> {
        self.space
            .iter()
            .find(|r| r.matches(a, b))
            .and_then(|r| r.result)
    }

    /// `Flow::joined`.
    pub fn joined(&self, a: &SyntaxToken, b: &SyntaxToken) -> bool {
        self.joined.iter().any(|r| r.matches(a, b))
    }

    /// `Flow::at_least_one`.
    pub fn at_least_one(&self, a: &SyntaxToken, b: &SyntaxToken) -> bool {
        self.at_least_one.iter().any(|r| r.matches(a, b))
    }

    /// `Flow::op_first`.
    pub fn op_first(&self, t: &SyntaxToken) -> bool {
        self.op_first.iter().any(|p| p.matches(t))
    }
}
