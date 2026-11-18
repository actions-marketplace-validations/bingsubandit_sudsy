//! Slot paths: how a rule row names a part of a node, read off the grammar's labels.
//!
//! A path is alternatives separated by `|` (the first that finds something wins); an alternative
//! is steps separated by `/` and an optional `:first`, `:last` or `:all` (which of the found parts;
//! `first` by default). A step is a label of the current node (`name`, `start/entries`) or, capitalized, the name of a kind of its children (`StageName`); `label[v]`
//! keeps the parts whose `key` slot has the text `v`; `.` is the node itself. The text of a part
//! is its source with blanks collapsed, and the text of `:all` runs from the first part to the
//! last.
//!
//! concept: none (glue: slot paths of rule rows)

use crate::tree::significant_range;
use core_syntax::{Grammar, SyntaxElement, SyntaxKind, SyntaxNode, TextRange};
use rowan::{GreenNodeData, GreenTokenData, NodeOrToken};
use std::borrow::Cow;
use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq)]
enum Pick {
    First,
    Last,
    All,
}

struct Step {
    label: String,
    key: Option<String>,
    /// The kind a capitalized label names, read from the grammar at the first use.
    kind: OnceLock<Option<SyntaxKind>>,
    /// Where the label and the `key` slot are in the kind of node they were first read from.
    label_at: OnceLock<(SyntaxKind, Option<Fixed>)>,
    key_at: OnceLock<(SyntaxKind, Option<Fixed>)>,
}

/// A label at a fixed child position of a kind of node: how many children that are not skipped
/// come before it, and the kinds it takes.
struct Fixed {
    nth: usize,
    accepted: Vec<SyntaxKind>,
}

impl Fixed {
    fn of(grammar: &Grammar, kind: SyntaxKind, label: &str) -> Option<Fixed> {
        let at = grammar.field_at(kind, label)?;
        Some(Fixed {
            nth: at.fixed()?,
            accepted: grammar.accepted(kind, at).to_vec(),
        })
    }

    /// The child at the position (`None` when it is not one the label takes), with its start.
    fn child<'a>(
        &self,
        grammar: &Grammar,
        node: &'a GreenNodeData,
        start: u32,
    ) -> Option<(u32, Green<'a>)> {
        let mut seen = 0;
        for (at, c) in green_children(node, start) {
            let k = green_kind(c);
            if grammar.is_skipped(k) {
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

/// `cell`'s [`Fixed`] of `label` in `kind`, looked up once for the first kind seen.
fn fixed_of<'c>(
    cell: &'c OnceLock<(SyntaxKind, Option<Fixed>)>,
    grammar: &Grammar,
    kind: SyntaxKind,
    label: &str,
    other: &'c mut Option<Fixed>,
) -> Option<&'c Fixed> {
    let (first, fixed) = cell.get_or_init(|| (kind, Fixed::of(grammar, kind, label)));
    if *first == kind {
        fixed.as_ref()
    } else {
        *other = Fixed::of(grammar, kind, label);
        other.as_ref()
    }
}

impl Step {
    fn new(label: &str, key: Option<&str>) -> Step {
        Step {
            label: label.to_string(),
            key: key.map(str::to_string),
            kind: OnceLock::new(),
            label_at: OnceLock::new(),
            key_at: OnceLock::new(),
        }
    }

    /// The kind this step selects children by, when its label is a kind name.
    fn by_kind(&self, grammar: &Grammar) -> Option<SyntaxKind> {
        *self.kind.get_or_init(|| {
            grammar
                .kind_by_name(&self.label)
                .filter(|_| self.label.starts_with(char::is_uppercase))
        })
    }
}

struct Alt {
    steps: Vec<Step>,
    pick: Pick,
}

pub struct SlotPath(Vec<Alt>);

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
                _ => (alt, Pick::First),
            };
            let steps = steps
                .split('/')
                .filter(|s| *s != ".")
                .map(
                    |s| match s.strip_suffix(']').and_then(|s| s.split_once('[')) {
                        Some((label, key)) => Step::new(label, Some(key)),
                        None => Step::new(s, None),
                    },
                )
                .collect();
            alts.push(Alt { steps, pick });
        }
        Ok(SlotPath(alts))
    }

    /// What the path finds in `node`. No allocation unless the text of a part spans blanks.
    pub fn find<'t>(
        &self,
        grammar: &Grammar,
        node: &SyntaxNode,
        text: &'t str,
    ) -> Option<Found<'t>> {
        self.0.iter().find_map(|alt| {
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
                Pick::First | Pick::All => (first?, last?),
                Pick::Last => (last.clone()?, last?),
            };
            let range = TextRange::new(span(grammar, &first).start(), span(grammar, &last).end());
            let source = &text[range];
            let text = if source.contains(char::is_whitespace) {
                Cow::Owned(source.split_whitespace().collect::<Vec<_>>().join(" "))
            } else {
                Cow::Borrowed(source)
            };
            Some(Found { text, range })
        })
    }
}

type Green<'a> = NodeOrToken<&'a GreenNodeData, &'a GreenTokenData>;

fn green_kind(c: Green<'_>) -> SyntaxKind {
    match c {
        NodeOrToken::Node(n) => SyntaxKind(n.kind().0),
        NodeOrToken::Token(t) => SyntaxKind(t.kind().0),
    }
}

/// The children of a green node with their start offsets.
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

impl SlotPath {
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
        for alt in &self.0 {
            let (mut first, mut last) = (None, None);
            let stop = alt.pick == Pick::First;
            visit_green(grammar, node, start, &alt.steps, text, &mut |at, len| {
                first.get_or_insert((at, len));
                last = Some((at, len));
                stop
            })?;
            let (first, last) = match alt.pick {
                Pick::First | Pick::All => (first, last),
                Pick::Last => (last, last),
            };
            if let (Some(first), Some(last)) = (first, last) {
                let range = TextRange::new(first.0.into(), (last.0 + last.1).into());
                let source = &text[range];
                let text = if source.contains(char::is_whitespace) {
                    Cow::Owned(source.split_whitespace().collect::<Vec<_>>().join(" "))
                } else {
                    Cow::Borrowed(source)
                };
                return Some(Some(Found { text, range }));
            }
        }
        Some(None)
    }
}

/// [`visit`] on green nodes; the parts are `(start, length)` of tokens. `None`: not possible here.
fn visit_green(
    grammar: &Grammar,
    node: &GreenNodeData,
    start: u32,
    steps: &[Step],
    text: &str,
    found: &mut dyn FnMut(u32, u32) -> bool,
) -> Option<bool> {
    let (step, rest) = steps.split_first()?;
    let mut visit_child = |at: u32, child: Green<'_>| -> Option<bool> {
        if let Some(key) = &step.key {
            let NodeOrToken::Node(part) = child else {
                return None;
            };
            let kind = SyntaxKind(part.kind().0);
            let mut other = None;
            let fixed = fixed_of(&step.key_at, grammar, kind, "key", &mut other)?;
            let (kat, kc) = fixed.child(grammar, part, at)?;
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
            (false, NodeOrToken::Node(n)) => visit_green(grammar, n, at, rest, text, found),
            (false, NodeOrToken::Token(_)) => Some(false),
        }
    };
    match step.by_kind(grammar) {
        Some(k) => {
            for (at, child) in green_children(node, start) {
                if green_kind(child) == k && visit_child(at, child)? {
                    return Some(true);
                }
            }
            Some(false)
        }
        None => {
            let kind = SyntaxKind(node.kind().0);
            let mut other = None;
            let fixed = fixed_of(&step.label_at, grammar, kind, &step.label, &mut other)?;
            let (at, child) = fixed.child(grammar, node, start)?;
            visit_child(at, child)
        }
    }
}

/// Calls `found` with each part the steps name below `node`, in order, until it returns true.
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
    match step.by_kind(grammar) {
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
