//! The shape of a data file (a workflow, a compose file, a manifest): per key path, which keys a
//! mapping may have, which it must have, what kind its value is, which names or text it may hold
//! and whether it may be empty. The language lists the rows (facts); the walk, the messages and
//! the formats (a cron line) are here. Rows are matched like the key paths of `embed_keys`: `*` is
//! any one key or index, the first row that matches a path (and whose variant the node has) is the row of
//! its node. A scalar that holds an `${{ }}`-style expression (the language's `expression`
//! marker) is not checked, its value is only known at run time.
//!
//! concept: key-shape
//! what: The allowed keys, required keys, kinds and values of the nodes of a data file, by key path.
//! tools: actionlint syntax-check, events, permissions

use crate::hooks::RuleHooks;
use crate::rule::FileContext;
use core_syntax::{Grammar, SyntaxElement, SyntaxNode, SyntaxNodeExt, TextRange};
use std::cell::RefCell;
use std::rc::Rc;

mod combos;
mod format;
mod typed;
pub use combos::Combos;
pub use format::Format;
pub use typed::{Typed, Value};

/// What a node must be.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// No check of its own (its names, keys or children are checked).
    Any,
    Map,
    Seq,
    /// A scalar of any type (what an `env` value or a `with` value is).
    Scalar,
    Str,
    Bool,
    Int,
    Float,
}

impl Kind {
    fn word(self) -> &'static str {
        match self {
            Kind::Any => "value",
            Kind::Map => "mapping",
            Kind::Seq => "sequence",
            Kind::Scalar => "scalar",
            Kind::Str => "string",
            Kind::Bool => "boolean",
            Kind::Int => "integer",
            Kind::Float => "number",
        }
    }
}

/// The facts about the nodes at one key path (`a.b|c.d.*`: either path).
pub struct Row {
    /// `jobs.*.steps.*`; empty is the root.
    pub path: &'static str,
    /// The rule that reports this row's findings.
    pub rule: &'static str,
    pub kind: Kind,
    /// What the node is called in messages (`job`); empty: its last key.
    pub title: &'static str,
    /// A mapping's allowed keys; empty is any key.
    pub keys: &'static [&'static str],
    pub required: &'static [&'static str],
    /// Pairs of keys that do not go together.
    pub exclusive: &'static [(&'static str, &'static str)],
    /// The scalar value or each scalar item must be one of these (empty: any).
    pub names: &'static [&'static str],
    pub format: Format,
    /// The node is a mapping that has (`true`) or lacks (`false`) this key.
    pub variant: &'static str,
    /// An empty mapping (`{}`) is fine; null, an empty string and an empty sequence still are errors.
    pub empty_map_ok: bool,
    /// A key repeated in the mapping, whatever the case, is an error.
    pub unique: bool,
    /// A null, empty string, empty mapping or empty sequence is an error.
    pub non_empty: bool,
    /// The entries of the mapping declare a type, a default and options.
    pub typed: Option<&'static Typed>,
    /// The mapping lists values per key, with include and exclude entries.
    pub combos: Option<&'static Combos>,
}

impl Row {
    pub const NONE: Row = Row {
        path: "",
        rule: "",
        kind: Kind::Any,
        title: "",
        keys: &[],
        required: &[],
        exclusive: &[],
        names: &[],
        format: Format::None,
        variant: "",
        unique: false,
        empty_map_ok: false,
        non_empty: false,
        typed: None,
        combos: None,
    };
}

/// A kind of mapping told apart by which keys it has: the first variant whose keys are all
/// present and whose `lacks` keys are all absent is the mapping's variant. A row with a `variant`
/// applies to the mappings of that variant only; a row without one applies to any.
pub struct Variant {
    pub name: &'static str,
    pub has: &'static [&'static str],
    pub lacks: &'static [&'static str],
}

pub struct Shape {
    pub variants: &'static [Variant],
    pub rows: &'static [Row],
    /// A scalar with this text inside is an expression: not checked.
    pub expression: &'static str,
    /// The rule that reports the structure of every row (unexpected or missing keys, a wrong kind,
    /// an empty section, a repeated key); empty: the row's own rule.
    pub structure: &'static str,
}

/// One problem found.
pub struct Finding {
    pub rule: &'static str,
    pub range: TextRange,
    pub message: String,
}

enum Val {
    Map(SyntaxNode),
    Seq(SyntaxNode),
    Scalar { text: String, quoted: bool },
    Null,
    Other,
}

struct Walk<'a> {
    g: &'a Grammar,
    hooks: &'a dyn RuleHooks,
    text: &'a str,
    shape: &'static Shape,
    patterns: Vec<Vec<Vec<&'static str>>>,
    out: Vec<Finding>,
}

fn is_match(pattern: &[&str], path: &[String]) -> bool {
    pattern.len() == path.len() && pattern.iter().zip(path).all(|(p, s)| *p == "*" || p == s)
}

impl Walk<'_> {
    fn inner(&self, node: &SyntaxNode) -> SyntaxNode {
        match self.g.slot(node, "content") {
            Some(SyntaxElement::Node(n)) => self.inner(&n),
            _ => node.clone(),
        }
    }

    fn val(&self, node: Option<&SyntaxNode>) -> Val {
        let Some(node) = node else { return Val::Null };
        let node = self.inner(node);
        let tag = |t| self.g.has_tag(node.kind(), t);
        if tag("mapping") {
            Val::Map(node)
        } else if tag("sequence") {
            Val::Seq(node)
        } else if let Some(token) = node.tokens().find(|t| self.g.has_tag(t.kind(), "string")) {
            let raw = &self.text[token.text_range()];
            let element = SyntaxElement::Node(node.clone());
            let text = self
                .hooks
                .key_name(self.g, &element, self.text)
                .into_owned();
            let solid = |t: &core_syntax::SyntaxToken| !self.g.has_tag(t.kind(), "trivia");
            let quoted = raw.starts_with(['\'', '"']) || node.tokens().filter(solid).count() > 1;
            Val::Scalar { text, quoted }
        } else if node.tokens().count() == 0 && node.children().count() == 0 {
            Val::Null
        } else {
            Val::Other
        }
    }

    fn entries(&self, map: &SyntaxNode) -> Vec<(String, TextRange, Option<SyntaxNode>)> {
        let mut out = Vec::new();
        for entry in map
            .children()
            .filter(|c| self.g.has_tag(c.kind(), "key-value"))
        {
            let Some(key) = self.g.slot(&entry, "key") else {
                continue;
            };
            let name = self.hooks.key_name(self.g, &key, self.text).into_owned();
            let range = match &key {
                SyntaxElement::Node(n) => crate::tree::significant_range(n, self.g),
                SyntaxElement::Token(t) => t.text_range(),
            };
            let value = self.g.slot(&entry, "value").and_then(|v| v.into_node());
            out.push((name, range, value));
        }
        out
    }

    fn items(&self, seq: &SyntaxNode) -> Vec<(TextRange, Option<SyntaxNode>)> {
        seq.children()
            .map(|item| {
                let value = if self.g.has_tag(item.kind(), "value") {
                    Some(item.clone())
                } else {
                    self.g.slot(&item, "value").and_then(|v| v.into_node())
                };
                let range = crate::tree::significant_range(value.as_ref().unwrap_or(&item), self.g);
                (range, value)
            })
            .collect()
    }

    fn report(&mut self, row: &Row, range: TextRange, message: String) {
        self.out.push(Finding {
            rule: row.rule,
            range,
            message,
        });
    }

    fn report_structure(&mut self, row: &Row, range: TextRange, message: String) {
        let rule = if self.shape.structure.is_empty() {
            row.rule
        } else {
            self.shape.structure
        };
        self.out.push(Finding {
            rule,
            range,
            message,
        });
    }

    fn is_expression(&self, text: &str) -> bool {
        !self.shape.expression.is_empty() && text.contains(self.shape.expression)
    }

    /// The variant of a mapping: the first whose key conditions hold.
    fn variant(&self, val: &Val) -> &'static str {
        let Val::Map(map) = val else { return "" };
        let entries = self.entries(map);
        let has = |k: &str| entries.iter().any(|e| e.0 == k);
        (self.shape.variants.iter())
            .find(|v| v.has.iter().all(|k| has(k)) && !v.lacks.iter().any(|k| has(k)))
            .map_or("", |v| v.name)
    }

    /// The row of the node at `path` for its variant.
    fn row(&self, path: &[String], val: &Val) -> Option<&'static Row> {
        let variant = self.variant(val);
        (self.shape.rows.iter().zip(&self.patterns))
            .find(|(row, alternatives)| {
                (row.variant.is_empty() || row.variant == variant)
                    && alternatives.iter().any(|p| is_match(p, path))
            })
            .map(|(row, _)| row)
    }

    fn wanted(&self, path: &[String]) -> bool {
        (self.patterns.iter().flatten())
            .any(|p| p.len() >= path.len() && p.iter().zip(path).all(|(a, b)| *a == "*" || a == b))
    }

    fn visit(&mut self, node: Option<SyntaxNode>, key: TextRange, path: &mut Vec<String>) {
        let val = self.val(node.as_ref());
        let at = match &node {
            None => key,
            Some(n) => crate::tree::significant_range(n, self.g),
        };
        if let Some(row) = self.row(path, &val) {
            self.check(row, &val, (key, at), path);
        }
        match val {
            Val::Map(map) => {
                for (name, range, value) in self.entries(&map) {
                    path.push(name);
                    if self.wanted(path) {
                        self.visit(value, range, path);
                    }
                    path.pop();
                }
            }
            Val::Seq(seq) => {
                for (i, (range, value)) in self.items(&seq).into_iter().enumerate() {
                    path.push(i.to_string());
                    if self.wanted(path) {
                        self.visit(value, range, path);
                    }
                    path.pop();
                }
            }
            _ => {}
        }
    }

    fn name(row: &Row, path: &[String]) -> String {
        if !row.title.is_empty() {
            return row.title.to_string();
        }
        let name = path.iter().rev().find(|s| s.parse::<usize>().is_err());
        name.cloned().unwrap_or_default()
    }

    fn check(
        &mut self,
        row: &'static Row,
        val: &Val,
        (key, at): (TextRange, TextRange),
        path: &[String],
    ) {
        let name = Self::name(row, path);
        let in_list = path.last().is_some_and(|s| s.parse::<usize>().is_ok());
        let empty = match val {
            Val::Null => true,
            Val::Scalar { text, quoted } => text.is_empty() && (*quoted || row.kind == Kind::Str),
            Val::Map(m) => self.entries(m).is_empty() && !row.empty_map_ok,
            Val::Seq(s) => self.items(s).is_empty(),
            Val::Other => false,
        };
        if let (Kind::Map, Val::Null, false) = (row.kind, val, row.non_empty) {
            self.check_map(row, None, key, path, &name);
        }
        if empty && row.non_empty {
            let scalar = !matches!(val, Val::Map(_) | Val::Seq(_))
                && !matches!(row.kind, Kind::Map | Kind::Seq);
            let message = match (scalar, in_list) {
                (true, _) => "string should not be empty".to_string(),
                (_, true) => format!("element of \"{name}\" section should not be empty"),
                _ => format!("\"{name}\" section should not be empty"),
            };
            self.report_structure(row, at, message);
            match (row.kind, val) {
                (Kind::Map, Val::Null) => self.check_map(row, None, key, path, &name),
                (Kind::Map, Val::Map(m)) => self.check_map(row, Some(m), key, path, &name),
                _ => {}
            }
            return;
        }
        if let Val::Scalar { text, .. } = val
            && self.is_expression(text)
        {
            return;
        }
        let ok = match (row.kind, val) {
            (Kind::Any, _) => true,
            (Kind::Map, Val::Map(_)) | (Kind::Seq, Val::Seq(_)) => true,

            (_, Val::Null) => true,
            (Kind::Scalar | Kind::Str, Val::Scalar { .. }) => true,
            (
                Kind::Bool,
                Val::Scalar {
                    text,
                    quoted: false,
                },
            ) => matches!(text.as_str(), "true" | "false"),
            (
                Kind::Int,
                Val::Scalar {
                    text,
                    quoted: false,
                },
            ) => text.parse::<i64>().is_ok(),
            (
                Kind::Float,
                Val::Scalar {
                    text,
                    quoted: false,
                },
            ) => text.parse::<f64>().is_ok(),
            (_, Val::Other) => true,
            _ => false,
        };
        if !ok {
            let got = match val {
                Val::Map(_) => "mapping",
                Val::Seq(_) => "sequence",
                _ => "scalar",
            };
            let message = format!("\"{name}\" must be {} but got {got}", row.kind.word());
            return self.report_structure(row, at, message);
        }
        match val {
            Val::Map(map) => self.check_map(row, Some(map), key, path, &name),
            Val::Scalar { text, .. } => self.check_names(row, std::iter::once((text.as_str(), at))),
            Val::Seq(seq) => {
                let items = self.items(seq);
                let texts: Vec<(String, TextRange)> = items
                    .iter()
                    .filter_map(|(r, v)| match self.val(v.as_ref()) {
                        Val::Scalar { text, .. } => Some((text, *r)),
                        _ => None,
                    })
                    .collect();
                self.check_names(row, texts.iter().map(|(t, r)| (t.as_str(), *r)));
            }
            _ => {}
        }
        if let Val::Scalar { text, .. } = val
            && let Some(message) = row.format.check(text)
        {
            self.report(row, at, message);
        }
        if let Val::Scalar { text, .. } = val
            && row.format == Format::Expression
            && !text.is_empty()
            && !self.is_expression(text)
        {
            let message = format!(
                "\"{name}\" should be specified via an expression such as a secret: do not put the value directly"
            );
            self.report(row, at, message);
        }
    }

    fn check_names<'t>(
        &mut self,
        row: &'static Row,
        names: impl Iterator<Item = (&'t str, TextRange)>,
    ) {
        if row.names.is_empty() {
            return;
        }
        for (text, range) in names {
            if !self.is_expression(text) && !row.names.contains(&text) {
                let message = format!(
                    "unknown name \"{text}\"; expected one of {}",
                    row.names.join(", ")
                );
                self.report(row, range, message);
            }
        }
    }

    fn check_map(
        &mut self,
        row: &'static Row,
        map: Option<&SyntaxNode>,
        at: TextRange,
        path: &[String],
        name: &str,
    ) {
        let entries = map.map(|m| self.entries(m)).unwrap_or_default();
        if !row.keys.is_empty() {
            for (key, range, _) in &entries {
                if !row.keys.contains(&key.as_str()) {
                    let message = format!(
                        "unexpected key \"{key}\" for {name} section. expected one of {}",
                        row.keys.join(", ")
                    );
                    self.report_structure(row, *range, message);
                }
            }
        }
        for key in row.required {
            if !entries.iter().any(|e| e.0 == *key) {
                let at = match map {
                    Some(map) if path.is_empty() => first_line(map, self.g),
                    _ => at,
                };
                self.report_structure(row, at, format!("\"{key}\" section is missing in {name}"));
            }
        }
        if row.unique {
            let mut seen: Vec<String> = Vec::new();
            for (key, range, _) in &entries {
                let lower = key.to_lowercase();
                if seen.contains(&lower) {
                    self.report_structure(
                        row,
                        *range,
                        format!("key \"{key}\" is duplicated in \"{name}\" section"),
                    );
                } else {
                    seen.push(lower);
                }
            }
        }
        if let Some(combos) = row.combos {
            self.combos(row, combos, &entries);
        }
        if let Some(typed) = row.typed {
            self.typed(row, typed, &entries);
        }
        for (a, b) in row.exclusive {
            let find = |k: &str| entries.iter().position(|e| e.0 == k);
            if let (Some(x), Some(y)) = (find(a), find(b)) {
                let message = format!("both \"{a}\" and \"{b}\" cannot be used in {name}");
                self.report(row, entries[x.max(y)].1, message);
            }
        }
    }
}

fn first_line(map: &SyntaxNode, g: &Grammar) -> TextRange {
    let range = crate::tree::significant_range(map, g);
    TextRange::new(range.start(), range.start())
}

type Cached = (SyntaxNode, usize, Rc<Vec<Finding>>);

thread_local! {
    /// The findings of the file read last: each rule of the shape asks for them in turn.
    static LAST: RefCell<Option<Cached>> = const { RefCell::new(None) };
}

/// Every problem of the file against `shape`, found once for the file read last.
pub fn findings(shape: &'static Shape, file: &FileContext<'_>) -> Rc<Vec<Finding>> {
    let key = std::ptr::from_ref(shape) as usize;
    LAST.with_borrow_mut(|last| {
        if let Some((root, k, found)) = last
            && *k == key
            && root == file.root
        {
            return found.clone();
        }
        let found = Rc::new(search(shape, file));
        *last = Some((file.root.clone(), key, found.clone()));
        found
    })
}

fn search(shape: &'static Shape, file: &FileContext<'_>) -> Vec<Finding> {
    let g = file.grammar;
    let mut body = file.root.clone();
    while !(g.has_tag(body.kind(), "mapping") || g.has_tag(body.kind(), "sequence")) {
        match body.children().next() {
            Some(n) => body = n,
            None => return Vec::new(),
        }
    }
    let mut walk = Walk {
        g,
        hooks: file.hooks,
        text: file.text,
        shape,
        patterns: (shape.rows.iter())
            .map(|r| {
                r.path
                    .split('|')
                    .map(|p| p.split('.').filter(|s| !s.is_empty()).collect())
                    .collect()
            })
            .collect(),
        out: Vec::new(),
    };
    walk.visit(Some(body), TextRange::default(), &mut Vec::new());
    walk.out
}

/// Reports the findings of `rule` (the rule's id) in the file.
pub fn check(
    shape: &'static Shape,
    rule: &str,
    file: &FileContext<'_>,
    ctx: &mut crate::RuleContext<'_>,
) {
    for finding in findings(shape, file).iter().filter(|f| f.rule == rule) {
        ctx.report(finding.range, finding.message.clone());
    }
}

/// A scalar of the file with its key path (sequence items are their index).
pub struct Leaf {
    pub path: Vec<String>,
    /// The scalar's text token.
    pub range: TextRange,
}

static NO_ROWS: Shape = Shape {
    variants: &[],
    rows: &[],
    expression: "",
    structure: "",
};

impl Walk<'_> {
    fn collect(
        &self,
        node: Option<SyntaxNode>,
        path: &mut Vec<String>,
        want: &dyn Fn(&[String], &str) -> bool,
        out: &mut Vec<Leaf>,
    ) {
        let Some(node) = node else { return };
        let node = self.inner(&node);
        let g = self.g;
        if g.has_tag(node.kind(), "mapping") {
            for entry in node.children().filter(|c| g.has_tag(c.kind(), "key-value")) {
                let Some(key) = g.slot(&entry, "key") else {
                    continue;
                };
                path.push(self.hooks.key_name(g, &key, self.text).into_owned());
                self.collect(
                    g.slot(&entry, "value").and_then(|v| v.into_node()),
                    path,
                    want,
                    out,
                );
                path.pop();
            }
        } else if g.has_tag(node.kind(), "sequence") {
            for (i, item) in node.children().enumerate() {
                let value = if g.has_tag(item.kind(), "value") {
                    Some(item.clone())
                } else {
                    g.slot(&item, "value").and_then(|v| v.into_node())
                };
                path.push(i.to_string());
                self.collect(value, path, want, out);
                path.pop();
            }
        } else if let Some(token) = node.tokens().find(|t| g.has_tag(t.kind(), "string"))
            && want(path, &self.text[token.text_range()])
        {
            out.push(Leaf {
                path: path.clone(),
                range: token.text_range(),
            });
        }
    }
}

/// The scalars of the file with their key path, those `want(path, text)` accepts.
pub fn leaves(file: &FileContext<'_>, want: &dyn Fn(&[String], &str) -> bool) -> Vec<Leaf> {
    let g = file.grammar;
    let mut body = file.root.clone();
    while !(g.has_tag(body.kind(), "mapping") || g.has_tag(body.kind(), "sequence")) {
        match body.children().next() {
            Some(n) => body = n,
            None => return Vec::new(),
        }
    }
    let walk = Walk {
        g,
        hooks: file.hooks,
        text: file.text,
        shape: &NO_ROWS,
        patterns: Vec::new(),
        out: Vec::new(),
    };
    let mut out = Vec::new();
    walk.collect(Some(body), &mut Vec::new(), want, &mut out);
    out
}

/// The values of a file read into a tree, for rules that look at other keys than their own.
pub enum Tree {
    Null,
    /// The text, and whether it was quoted.
    Scalar(String, bool),
    Map(Vec<(String, Tree)>),
    Seq(Vec<Tree>),
}

impl Tree {
    pub fn get(&self, key: &str) -> Option<&Tree> {
        match self {
            Tree::Map(m) => m.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// The value at `path` (keys; sequence items by index).
    pub fn at<S: AsRef<str>>(&self, path: &[S]) -> Option<&Tree> {
        let mut node = self;
        for key in path {
            let key = key.as_ref();
            node = match node {
                Tree::Seq(items) => items.get(key.parse::<usize>().ok()?)?,
                _ => node.get(key)?,
            };
        }
        Some(node)
    }
}

impl Walk<'_> {
    fn tree(&self, node: Option<SyntaxNode>) -> Tree {
        match self.val(node.as_ref()) {
            Val::Map(map) => Tree::Map(
                (self.entries(&map).into_iter())
                    .map(|(name, _, value)| (name, self.tree(value)))
                    .collect(),
            ),
            Val::Seq(seq) => Tree::Seq(
                (self.items(&seq).into_iter())
                    .map(|(_, value)| self.tree(value))
                    .collect(),
            ),
            Val::Scalar { text, quoted } => Tree::Scalar(text, quoted),
            _ => Tree::Null,
        }
    }
}

/// The whole file as a tree.
pub fn tree(file: &FileContext<'_>) -> Tree {
    let g = file.grammar;
    let mut body = file.root.clone();
    while !(g.has_tag(body.kind(), "mapping") || g.has_tag(body.kind(), "sequence")) {
        match body.children().next() {
            Some(n) => body = n,
            None => return Tree::Null,
        }
    }
    let walk = Walk {
        g,
        hooks: file.hooks,
        text: file.text,
        shape: &NO_ROWS,
        patterns: Vec::new(),
        out: Vec::new(),
    };
    walk.tree(Some(body))
}
