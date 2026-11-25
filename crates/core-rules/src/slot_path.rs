//! Slot paths: how a rule row names a part of a node, read off the grammar's labels.
//!
//! A path is alternatives separated by `|` (the first that finds something wins); an alternative
//! is steps separated by `/` and an optional `:first`, `:last`, `:all` or `:each` (which of the found parts;
//! `first` by default; `each` is the same as `all` for [`SlotPath::find`] and makes
//! [`SlotPath::find_each`] report every part). A step is a label of the current node (`name`, `start/entries`) or, capitalized, the name of a kind of its children (`StageName`); `label[v]`
//! keeps the parts whose `key` slot has the text `v`; `.` is the node itself. The text of a part
//! is its source with blanks collapsed, and the text of `:all` runs from the first part to the
//! last.
//!
//! A path is read against the grammar once, at its first use: each label becomes a table by kind
//! of node (child position and accepted kinds), so a walk of the green tree is raw kind compares
//! with no lookups by name.
//!
//! concept: none (glue: slot paths of rule rows)

use crate::tree::significant_range;
use core_syntax::{Grammar, LanguageId, SyntaxElement, SyntaxKind, SyntaxNode, TextRange};
use rowan::{GreenNodeData, GreenTokenData, NodeOrToken};
use std::borrow::Cow;
use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq)]
enum Pick {
    First,
    Last,
    All,
    Each,
}

/// A label at a fixed child position of a kind of node: how many children that are not skipped
/// come before it, and the kinds it takes.
struct Fixed {
    nth: usize,
    accepted: Vec<SyntaxKind>,
}

/// Per node kind of the grammar (by `SyntaxKind::index`): where a label is; `None` when the kind
/// has no such label or it is not at a fixed position.
type FixedByKind = Vec<Option<Fixed>>;

fn fixed_table(grammar: &Grammar, label: &str) -> FixedByKind {
    grammar
        .nodes()
        .iter()
        .map(|def| {
            let at = grammar.field_at(def.kind, label)?;
            Some(Fixed {
                nth: at.fixed()?,
                accepted: grammar.accepted(def.kind, at).to_vec(),
            })
        })
        .collect()
}

/// The kinds that take no child position (trivia, `ERROR`) as flags by kind index, and the
/// language the tables are of.
struct Skip {
    lang: LanguageId,
    tokens: Vec<bool>,
    nodes: Vec<bool>,
}

impl Skip {
    fn of(grammar: &Grammar) -> Skip {
        Skip {
            lang: grammar.root().lang(),
            tokens: grammar
                .tokens()
                .iter()
                .map(|d| grammar.is_skipped(d.kind))
                .collect(),
            nodes: grammar
                .nodes()
                .iter()
                .map(|d| grammar.is_skipped(d.kind))
                .collect(),
        }
    }

    #[inline]
    fn has(&self, kind: SyntaxKind) -> bool {
        let flags = if kind.is_token() {
            &self.tokens
        } else {
            &self.nodes
        };
        flags.get(kind.index() as usize).copied().unwrap_or(false)
    }
}

impl Fixed {
    /// The child at the position (`None` when it is not one the label takes), with its start.
    #[inline]
    fn child<'a>(
        &self,
        skip: &Skip,
        node: &'a GreenNodeData,
        start: u32,
    ) -> Option<(u32, Green<'a>)> {
        let mut seen = 0;
        for (at, c) in green_children(node, start) {
            let k = green_kind(c);
            if skip.has(k) {
                continue;
            }
            if seen < self.nth {
                seen += 1;
                continue;
            }
            return (k != SyntaxKind::ERROR && self.accepted.contains(&k)).then_some((at, c));
        }
        None
    }
}

/// One step of a path, read against the grammar.
struct Step {
    label: String,
    key: Option<String>,
    /// The kind a capitalized label names: the step takes the children of that kind.
    kind: Option<SyntaxKind>,
    /// Where the label and the `key` slot are in each kind of node (empty for a step by kind and
    /// for no `key`).
    label_at: FixedByKind,
    key_at: FixedByKind,
}

impl Step {
    fn resolve(grammar: &Grammar, label: &str, key: Option<&str>) -> Step {
        let kind = grammar
            .kind_by_name(label)
            .filter(|_| label.starts_with(char::is_uppercase));
        Step {
            label: label.to_string(),
            key: key.map(str::to_string),
            kind,
            label_at: match kind {
                Some(_) => Vec::new(),
                None => fixed_table(grammar, label),
            },
            key_at: match key {
                Some(_) => fixed_table(grammar, "key"),
                None => Vec::new(),
            },
        }
    }

    /// Where the label (`key`: the `key` slot) is in nodes of `kind`; `None` when it has no fixed
    /// place there.
    #[inline]
    fn at(&self, skip: &Skip, kind: SyntaxKind, key: bool) -> Option<&Fixed> {
        if kind.lang() != skip.lang {
            return None;
        }
        let table = if key { &self.key_at } else { &self.label_at };
        table.get(kind.index() as usize)?.as_ref()
    }
}

struct Alt {
    steps: Vec<Step>,
    pick: Pick,
}

/// The paths of a spec read against the grammar.
struct Plan {
    alts: Vec<Alt>,
    skip: Skip,
}

type Spec = Vec<(Vec<(String, Option<String>)>, Pick)>;

pub struct SlotPath {
    spec: Spec,
    plan: OnceLock<Plan>,
}

/// A part found by a [`SlotPath`]: its text and where it is.
pub struct Found<'t> {
    pub text: Cow<'t, str>,
    pub range: TextRange,
}

impl SlotPath {
    pub fn parse(spec: &str) -> Result<SlotPath, String> {
        let mut alts = Vec::new();
        for alt in spec.split('|') {
            let (steps, pick) = match alt.rsplit_once(':') {
                Some((s, "first")) => (s, Pick::First),
                Some((s, "last")) => (s, Pick::Last),
                Some((s, "all")) => (s, Pick::All),
                Some((s, "each")) => (s, Pick::Each),
                _ => (alt, Pick::First),
            };
            let steps = steps
                .split('/')
                .filter(|s| *s != ".")
                .map(
                    |s| match s.strip_suffix(']').and_then(|s| s.split_once('[')) {
                        Some((label, key)) => (label.to_string(), Some(key.to_string())),
                        None => (s.to_string(), None),
                    },
                )
                .collect();
            alts.push((steps, pick));
        }
        Ok(SlotPath {
            spec: alts,
            plan: OnceLock::new(),
        })
    }

    fn plan(&self, grammar: &Grammar) -> &Plan {
        self.plan.get_or_init(|| Plan {
            alts: self
                .spec
                .iter()
                .map(|(steps, pick)| Alt {
                    steps: steps
                        .iter()
                        .map(|(label, key)| Step::resolve(grammar, label, key.as_deref()))
                        .collect(),
                    pick: *pick,
                })
                .collect(),
            skip: Skip::of(grammar),
        })
    }

    /// What the path finds in `node`. No allocation unless the text of a part spans blanks.
    pub fn find<'t>(
        &self,
        grammar: &Grammar,
        node: &SyntaxNode,
        text: &'t str,
    ) -> Option<Found<'t>> {
        let plan = self.plan(grammar);
        plan.alts
            .iter()
            .find_map(|alt| find_alt(grammar, alt, node, text))
    }

    /// Every part the first alternative that finds something names, one by one (`a, b := 1, 2`:
    /// `ExpressionList/Ident:each` finds `a` and `b`).
    pub fn find_each<'t>(
        &self,
        grammar: &Grammar,
        node: &SyntaxNode,
        text: &'t str,
    ) -> Vec<Found<'t>> {
        for alt in &self.plan(grammar).alts {
            if alt.pick != Pick::Each {
                if let Some(one) = find_alt(grammar, alt, node, text) {
                    return vec![one];
                }
                continue;
            }
            let mut parts = Vec::new();
            visit(grammar, node, &alt.steps, text, &mut |part| {
                parts.push(found(text, span(grammar, &part)));
                false
            });
            if !parts.is_empty() {
                return parts;
            }
        }
        Vec::new()
    }

    /// [`SlotPath::find`] on the green tree, so no red element is allocated: `None` when a step
    /// needs the red tree (a label with no fixed position, a part that is a node), `Some(None)`
    /// when the path finds nothing.
    pub fn find_green<'t>(
        &self,
        grammar: &Grammar,
        node: &GreenNodeData,
        start: u32,
        text: &'t str,
    ) -> Option<Option<Found<'t>>> {
        self.green_below(self.plan(grammar), node, start, text, 0)
    }

    /// Do `self` and `other` both go below the same plain label first (one alternative each, a
    /// label of the node and not a kind, no key, and more steps after it)? Then
    /// [`SlotPath::find_green_with`] reads that node once for both.
    pub fn shares_start(&self, other: &SlotPath) -> bool {
        match (&self.spec[..], &other.spec[..]) {
            ([(x, _)], [(y, _)]) => {
                x.len() > 1
                    && y.len() > 1
                    && x[0] == y[0]
                    && x[0].1.is_none()
                    && !x[0].0.starts_with(char::is_uppercase)
            }
            _ => false,
        }
    }

    /// [`SlotPath::find_green`] for two paths that [share their start](SlotPath::shares_start):
    /// the node of the first step is read once and both go on below it. `None` when a path needs
    /// the red tree; `Some(None)` when `self` finds nothing (`other` is then not read).
    #[allow(clippy::type_complexity)]
    pub fn find_green_with<'t>(
        &self,
        other: &SlotPath,
        grammar: &Grammar,
        node: &GreenNodeData,
        start: u32,
        text: &'t str,
    ) -> Option<Option<(Found<'t>, Option<Found<'t>>)>> {
        let (a, b) = (self.plan(grammar), other.plan(grammar));
        let first = &a.alts[0].steps[0];
        let fixed = first.at(&a.skip, SyntaxKind(node.kind().0), false)?;
        let (at, below) = fixed.child(&a.skip, node, start)?;
        let NodeOrToken::Node(below) = below else {
            return None;
        };
        let Some(name) = self.green_below(a, below, at, text, 1)? else {
            return Some(None);
        };
        Some(Some((name, other.green_below(b, below, at, text, 1)?)))
    }

    /// The first part of the path's alternatives, reading each from its step number `from`.
    fn green_below<'t>(
        &self,
        plan: &Plan,
        node: &GreenNodeData,
        start: u32,
        text: &'t str,
        from: usize,
    ) -> Option<Option<Found<'t>>> {
        for alt in &plan.alts {
            let (mut first, mut last) = (None, None);
            let stop = alt.pick == Pick::First;
            visit_green(
                &plan.skip,
                node,
                start,
                &alt.steps[from..],
                text,
                &mut |at, len| {
                    first.get_or_insert((at, len));
                    last = Some((at, len));
                    stop
                },
            )?;
            let (first, last) = match alt.pick {
                Pick::First | Pick::All | Pick::Each => (first, last),
                Pick::Last => (last, last),
            };
            if let (Some(first), Some(last)) = (first, last) {
                let range = TextRange::new(first.0.into(), (last.0 + last.1).into());
                return Some(Some(found(text, range)));
            }
        }
        Some(None)
    }
}

/// The part of `text` at `range`, blanks collapsed.
fn found(text: &str, range: TextRange) -> Found<'_> {
    let source = &text[range];
    // Blanks are rare: a byte scan finds the names that have none (ASCII) before any char decode.
    let maybe_blank = source.bytes().any(|b| b <= b' ' || b >= 0x80);
    let text = if maybe_blank && source.contains(char::is_whitespace) {
        Cow::Owned(source.split_whitespace().collect::<Vec<_>>().join(" "))
    } else {
        Cow::Borrowed(source)
    };
    Found { text, range }
}

type Green<'a> = NodeOrToken<&'a GreenNodeData, &'a GreenTokenData>;

#[inline]
fn green_kind(c: Green<'_>) -> SyntaxKind {
    match c {
        NodeOrToken::Node(n) => SyntaxKind(n.kind().0),
        NodeOrToken::Token(t) => SyntaxKind(t.kind().0),
    }
}

/// The children of a green node with their start offsets.
#[inline]
pub(crate) fn green_children(
    node: &GreenNodeData,
    start: u32,
) -> impl Iterator<Item = (u32, Green<'_>)> {
    let mut at = start;
    node.children().map(move |c| {
        let len = match c {
            NodeOrToken::Node(n) => u32::from(n.text_len()),
            NodeOrToken::Token(t) => u32::from(t.text_len()),
        };
        at += len;
        (at - len, c)
    })
}

/// Calls `found` with the `(start, length)` of each token part the steps name below `node`, in
/// order, until it returns true. `None`: not possible on the green tree here.
fn visit_green<F: FnMut(u32, u32) -> bool>(
    skip: &Skip,
    node: &GreenNodeData,
    start: u32,
    steps: &[Step],
    text: &str,
    found: &mut F,
) -> Option<bool> {
    let (step, rest) = steps.split_first()?;
    let mut visit_child = |at: u32, child: Green<'_>| -> Option<bool> {
        if let Some(key) = &step.key {
            let NodeOrToken::Node(part) = child else {
                return None;
            };
            let fixed = step.at(skip, SyntaxKind(part.kind().0), true)?;
            let (kat, kc) = fixed.child(skip, part, at)?;
            let NodeOrToken::Token(kt) = kc else {
                return None;
            };
            if &text[kat as usize..(kat + u32::from(kt.text_len())) as usize] != key {
                return Some(false);
            }
        }
        match (rest.is_empty(), child) {
            (true, NodeOrToken::Token(t)) => Some(found(at, u32::from(t.text_len()))),
            (true, NodeOrToken::Node(_)) => None,
            (false, NodeOrToken::Node(n)) => visit_green(skip, n, at, rest, text, found),
            (false, NodeOrToken::Token(_)) => Some(false),
        }
    };
    match step.kind {
        Some(k) => {
            for (at, child) in green_children(node, start) {
                if green_kind(child) == k && visit_child(at, child)? {
                    return Some(true);
                }
            }
            Some(false)
        }
        None => {
            let fixed = step.at(skip, SyntaxKind(node.kind().0), false)?;
            let (at, child) = fixed.child(skip, node, start)?;
            visit_child(at, child)
        }
    }
}

/// Calls `found` with each part the steps name below `node`, in order, until it returns true.
/// What one alternative finds in `node`.
fn find_alt<'t>(
    grammar: &Grammar,
    alt: &Alt,
    node: &SyntaxNode,
    text: &'t str,
) -> Option<Found<'t>> {
    let (mut first, mut last) = (None, None);
    let stop_at_first = alt.pick == Pick::First;
    visit(grammar, node, &alt.steps, text, &mut |part| {
        if first.is_none() {
            first = Some(part.clone());
        }
        last = Some(part);
        stop_at_first
    });
    let (first, last) = match alt.pick {
        Pick::First | Pick::All | Pick::Each => (first?, last?),
        Pick::Last => (last.clone()?, last?),
    };
    let range = TextRange::new(span(grammar, &first).start(), span(grammar, &last).end());
    Some(found(text, range))
}

fn visit(
    grammar: &Grammar,
    node: &SyntaxNode,
    steps: &[Step],
    text: &str,
    found: &mut dyn FnMut(SyntaxElement) -> bool,
) -> bool {
    let Some((step, rest)) = steps.split_first() else {
        return found(SyntaxElement::Node(node.clone()));
    };
    let part = |p: SyntaxElement| -> bool {
        if step
            .key
            .as_deref()
            .is_some_and(|k| !has_key(grammar, &p, k, text))
        {
            return false;
        }
        match (rest.is_empty(), p) {
            (true, p) => found(p),
            (false, SyntaxElement::Node(n)) => visit(grammar, &n, rest, text, found),
            (false, _) => false,
        }
    };
    match step.kind {
        Some(kind) => node
            .children_with_tokens()
            .filter(|c| c.kind() == kind)
            .any(part),
        None => grammar.slots(node, &step.label).any(part),
    }
}

fn span(grammar: &Grammar, part: &SyntaxElement) -> TextRange {
    match part {
        SyntaxElement::Token(t) => t.text_range(),
        SyntaxElement::Node(n) => significant_range(n, grammar),
    }
}

fn has_key(grammar: &Grammar, part: &SyntaxElement, key: &str, text: &str) -> bool {
    part.as_node()
        .and_then(|n| grammar.slot(n, "key"))
        .is_some_and(|k| &text[span(grammar, &k)] == key)
}
