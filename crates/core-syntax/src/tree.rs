//! The lossless tree: `rowan` green/red trees keyed by our [`SyntaxKind`].
//!
//! concept: none (infrastructure: lossless tree)

use crate::kind::SyntaxKind;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SudsyLang {}

impl rowan::Language for SudsyLang {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> SyntaxKind {
        SyntaxKind(raw.0)
    }

    fn kind_to_raw(kind: SyntaxKind) -> rowan::SyntaxKind {
        rowan::SyntaxKind(kind.0)
    }
}

pub type SyntaxNode = rowan::SyntaxNode<SudsyLang>;
pub type SyntaxToken = rowan::SyntaxToken<SudsyLang>;
pub type SyntaxElement = rowan::SyntaxElement<SudsyLang>;

/// Lookups on a node's direct children.
pub trait SyntaxNodeExt {
    /// The first child node of `kind`.
    fn child(&self, kind: SyntaxKind) -> Option<SyntaxNode>;
    /// The own tokens (not those of child nodes).
    fn tokens(&self) -> impl Iterator<Item = SyntaxToken>;
    /// The first own token of `kind`.
    fn token(&self, kind: SyntaxKind) -> Option<SyntaxToken> {
        self.tokens().find(|t| t.kind() == kind)
    }
}

impl SyntaxNodeExt for SyntaxNode {
    fn child(&self, kind: SyntaxKind) -> Option<SyntaxNode> {
        self.children().find(|c| c.kind() == kind)
    }

    fn tokens(&self) -> impl Iterator<Item = SyntaxToken> {
        self.children_with_tokens().filter_map(|e| e.into_token())
    }
}

/// The first token below `node` that is not trivia.
pub fn first_code_token(node: &SyntaxNode, grammar: &crate::Grammar) -> Option<SyntaxToken> {
    node.descendants_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| !grammar.is_trivia(t.kind()))
}

/// The span from the first to the last token of `node` that is neither trivia nor empty, `None`
/// without one. Walks the green tree, so no red element is allocated.
pub fn code_span(node: &SyntaxNode, grammar: &crate::Grammar) -> Option<rowan::TextRange> {
    use rowan::{GreenNodeData, NodeOrToken};
    fn edge(
        n: &GreenNodeData,
        grammar: &crate::Grammar,
        back: bool,
        at: &mut u32,
    ) -> Option<(u32, u32)> {
        let mut visit = |c: NodeOrToken<&GreenNodeData, &rowan::GreenTokenData>| match c {
            NodeOrToken::Node(n) => edge(n, grammar, back, at),
            NodeOrToken::Token(t) => {
                let len = u32::from(t.text_len());
                if back {
                    *at -= len;
                }
                let start = *at;
                if !back {
                    *at += len;
                }
                (len != 0 && !grammar.is_trivia(SyntaxKind(t.kind().0))).then_some((start, len))
            }
        };
        if back {
            n.children().rev().find_map(&mut visit)
        } else {
            n.children().find_map(&mut visit)
        }
    }
    let green = node.green();
    let whole = node.text_range();
    let (first, _) = edge(&green, grammar, false, &mut 0)?;
    let (last, len) = edge(&green, grammar, true, &mut u32::from(green.text_len()))?;
    let base = whole.start();
    Some(rowan::TextRange::new(
        base + rowan::TextSize::from(first),
        base + rowan::TextSize::from(last + len),
    ))
}

/// What [`find`] looks for, each a set by kind index.
#[derive(Default)]
pub struct Wanted {
    /// Node kinds: every node of one is found (the root included; nodes with no text are not).
    pub nodes: Vec<bool>,
    /// Node kinds whose direct child tokens of a kind of `tokens` are found.
    pub parents: Vec<bool>,
    pub tokens: Vec<bool>,
    /// Every token kind, whatever `tokens` says.
    pub any_token: bool,
}

/// The nodes and the tokens `wanted` names, in document order. Walks the green tree, so a red
/// element is made for a hit only.
pub fn find(root: &SyntaxNode, wanted: &Wanted) -> (Vec<SyntaxNode>, Vec<SyntaxToken>) {
    use rowan::{GreenNodeData, NodeOrToken};
    type Hit = (SyntaxKind, u32, u32, bool);
    fn walk(n: &GreenNodeData, at: u32, w: &Wanted, hits: &mut Vec<Hit>) {
        let has =
            |set: &[bool], k: SyntaxKind| set.get(k.index() as usize).copied().unwrap_or(false);
        let kind = SyntaxKind(n.kind().0);
        let len = u32::from(n.text_len());
        if len > 0 && has(&w.nodes, kind) {
            hits.push((kind, at, len, false));
        }
        let parent = has(&w.parents, kind);
        let mut at = at;
        for c in n.children() {
            match c {
                NodeOrToken::Node(c) => {
                    walk(c, at, w, hits);
                    at += u32::from(c.text_len());
                }
                NodeOrToken::Token(t) => {
                    let (k, l) = (SyntaxKind(t.kind().0), u32::from(t.text_len()));
                    if parent && l > 0 && (w.any_token || has(&w.tokens, k)) {
                        hits.push((k, at, l, true));
                    }
                    at += l;
                }
            }
        }
    }
    let mut hits = Vec::new();
    walk(
        &root.green(),
        u32::from(root.text_range().start()),
        wanted,
        &mut hits,
    );
    let (mut nodes, mut tokens) = (Vec::new(), Vec::new());
    for (kind, at, len, is_token) in hits {
        let range = rowan::TextRange::at(at.into(), len.into());
        match root.covering_element(range) {
            NodeOrToken::Token(t) if is_token && t.kind() == kind => tokens.push(t),
            NodeOrToken::Token(_) if is_token => {}
            e => {
                let Some(mut n) = (match e {
                    NodeOrToken::Node(n) => Some(n),
                    NodeOrToken::Token(t) => t.parent(),
                }) else {
                    continue;
                };
                while n.kind() != kind && n.parent().is_some_and(|p| p.text_range() == range) {
                    let Some(p) = n.parent() else { break };
                    n = p;
                }
                if n.kind() == kind && n.text_range() == range {
                    nodes.push(n);
                }
            }
        }
    }
    (nodes, tokens)
}

/// Prints the tree. The lossless invariant is `print(&tree) == source`.
pub fn print(node: &SyntaxNode) -> String {
    node.text().to_string()
}

/// Thin wrapper over `rowan::GreenNodeBuilder` that speaks our kinds.
#[derive(Default)]
pub struct TreeBuilder {
    inner: rowan::GreenNodeBuilder<'static>,
}

impl TreeBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start_node(&mut self, kind: SyntaxKind) {
        debug_assert!(!kind.is_token());
        self.inner.start_node(rowan::SyntaxKind(kind.0));
    }

    pub fn token(&mut self, kind: SyntaxKind, text: &str) {
        debug_assert!(kind.is_token());
        self.inner.token(rowan::SyntaxKind(kind.0), text);
    }

    pub fn finish_node(&mut self) {
        self.inner.finish_node();
    }

    /// The position before the next element, for [`Self::start_node_at`]: a parser that finds
    /// out after the fact that a node began earlier (a left operand) wraps what it built.
    pub fn checkpoint(&self) -> rowan::Checkpoint {
        self.inner.checkpoint()
    }

    /// Opens a node around everything built since `checkpoint`; the current node must be the one
    /// that was open when the checkpoint was taken.
    pub fn start_node_at(&mut self, checkpoint: rowan::Checkpoint, kind: SyntaxKind) {
        debug_assert!(!kind.is_token());
        self.inner
            .start_node_at(checkpoint, rowan::SyntaxKind(kind.0));
    }

    pub fn finish(self) -> rowan::GreenNode {
        self.inner.finish()
    }
}

/// Indented dump for debugging: `Kind start..end` for nodes, plus the escaped text for tokens.
pub fn dump_tree(root: &SyntaxNode, grammar: &crate::Grammar) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let mut depth = 0usize;
    for event in root.preorder_with_tokens() {
        match event {
            rowan::WalkEvent::Enter(el) => {
                let name = grammar.name(el.kind());
                let range = el.text_range();
                let (start, end) = (u32::from(range.start()), u32::from(range.end()));
                let _ = write!(
                    out,
                    "{:indent$}{name} {start}..{end}",
                    "",
                    indent = depth * 2
                );
                if let rowan::NodeOrToken::Token(t) = &el {
                    let _ = write!(out, " {:?}", t.text());
                }
                out.push('\n');
                depth += 1;
            }
            rowan::WalkEvent::Leave(_) => depth -= 1,
        }
    }
    out
}
