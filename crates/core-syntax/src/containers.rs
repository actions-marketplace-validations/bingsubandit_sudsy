//! The `container` rows of a grammar: nodes whose content lines all carry a prefix (a quote's
//! `>`, a list item's indentation), and the source lines of a fenced block inside them.
//!
//! `container <Node> marker <Token> [prefix "s"]`: the content of the node starts after its first
//! `marker` token; each following line carries `s` (and one space), or, without `prefix`, as much
//! indentation as the content is indented.
//!
//! concept: container-rows
//! what: Nodes whose content lines all carry a prefix (quote marker, list indent), and fenced blocks inside them.
//! tools: CommonMark container blocks
//! reads: grammar:container

use crate::SyntaxKind;
use crate::grammar::Grammar;
use crate::tree::{SyntaxNode, SyntaxNodeExt, SyntaxToken};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContainerRow {
    pub node: SyntaxKind,
    pub marker: SyntaxKind,
    pub prefix: Option<String>,
}

/// A container whose prefix every line of its content carries.
#[derive(Clone, Debug)]
pub enum Cont {
    Prefix(String),
    /// Width of the continuation indentation, relative to the container around it.
    Indent(usize),
}

/// Does a line still belong to the containers (a prefix needs its text, an indent its spaces or a
/// blank)?
pub fn in_container(line: &str, chain: &[Cont]) -> bool {
    let mut line = line;
    for c in chain {
        match c {
            Cont::Prefix(p) => {
                if !line.trim_start_matches(' ').starts_with(p.as_str()) {
                    return false;
                }
            }
            Cont::Indent(w) => {
                if !line.trim().is_empty() && line.bytes().take_while(|b| *b == b' ').count() < *w {
                    return false;
                }
            }
        }
        line = strip(line, std::slice::from_ref(c));
    }
    true
}

/// Removes the container prefix from a line.
pub fn strip<'a>(mut line: &'a str, chain: &[Cont]) -> &'a str {
    for c in chain {
        line = match c {
            Cont::Prefix(p) => {
                let t = line.trim_start_matches(' ');
                t.strip_prefix(p.as_str())
                    .map_or(line, |r| r.strip_prefix(' ').unwrap_or(r))
            }
            Cont::Indent(w) => {
                let n = line.bytes().take(*w).take_while(|b| *b == b' ').count();
                &line[n..]
            }
        };
    }
    line
}

/// The containers of `src`'s tree, read through the grammar's rows.
pub struct Containers<'a> {
    pub grammar: &'a Grammar,
    pub src: &'a str,
}

fn start(t: &SyntaxToken) -> usize {
    usize::from(t.text_range().start())
}

fn end(t: &SyntaxToken) -> usize {
    usize::from(t.text_range().end())
}

impl Containers<'_> {
    pub fn line_start(&self, off: usize) -> usize {
        self.src[..off].rfind('\n').map_or(0, |i| i + 1)
    }

    fn row(&self, node: &SyntaxNode) -> Option<&ContainerRow> {
        (self.grammar.containers().iter()).find(|r| r.node == node.kind())
    }

    /// Absolute column where the content of a container starts.
    pub fn content_col(&self, n: &SyntaxNode) -> usize {
        let marker = self.row(n).and_then(|r| n.token(r.marker));
        marker.map_or(0, |t| end(&t) - self.line_start(start(&t)))
    }

    /// The containers around `node`, outermost first.
    pub fn chain(&self, node: &SyntaxNode) -> Vec<Cont> {
        let mut out = Vec::new();
        let mut parent_col = None;
        let mut ancestors: Vec<SyntaxNode> = node
            .ancestors()
            .skip(1)
            .filter(|a| self.row(a).is_some())
            .collect();
        ancestors.reverse();
        for a in ancestors {
            let col = self.content_col(&a);
            out.push(self.cont(&a, col.saturating_sub(parent_col.unwrap_or(0))));
            parent_col = Some(col);
        }
        out
    }

    fn cont(&self, node: &SyntaxNode, width: usize) -> Cont {
        match self.row(node).and_then(|r| r.prefix.clone()) {
            Some(p) => Cont::Prefix(p),
            None => Cont::Indent(width),
        }
    }

    /// Chain for the content of `node` itself when it is a container.
    pub fn chain_in(&self, node: &SyntaxNode) -> Vec<Cont> {
        let mut c = self.chain(node);
        let parent_col = node
            .ancestors()
            .skip(1)
            .find(|a| self.row(a).is_some())
            .map_or(0, |a| self.content_col(&a));
        c.push(self.cont(node, self.content_col(node).saturating_sub(parent_col)));
        c
    }

    /// The containers around a token, those of the node it stands in included.
    pub fn chain_of_token(&self, t: &SyntaxToken) -> Vec<Cont> {
        t.parent().map_or(Vec::new(), |p| {
            if self.row(&p).is_some() {
                self.chain_in(&p)
            } else {
                self.chain(&p)
            }
        })
    }

    /// The content lines of the fenced block `n` (opened by its `fence` token) as `(line start,
    /// length of the container prefix and the fence's indentation, line end)`, and whether the
    /// closing fence was found. The content is read from the source, line by line up to the
    /// closing fence: blank content has no token, and the closing fence may stand outside the
    /// node.
    pub fn fence_lines(
        &self,
        n: &SyntaxNode,
        fence: SyntaxKind,
    ) -> (Vec<(usize, usize, usize)>, bool) {
        let chain = self.chain(n);
        let (mut lines, mut closed) = (Vec::new(), false);
        if let Some(open) = n.token(fence) {
            let text = open.text().trim();
            let (fch, flen) = (text.chars().next().unwrap_or('`'), text.len());
            // Indentation of the opening fence beyond its containers: content loses as much.
            let lead = open.text().len() - open.text().trim_start().len();
            let before = strip(
                &self.src[self.line_start(start(&open))..start(&open)],
                &chain,
            );
            let fence_indent = lead
                + if before.trim().is_empty() {
                    before.len()
                } else {
                    0
                };
            let from = self.src[end(&open)..]
                .find('\n')
                .map_or(self.src.len(), |i| end(&open) + i + 1);
            let mut pieces = self.src[from..].split('\n').peekable();
            let mut at = from;
            while let Some(piece) = pieces.next() {
                let raw = piece.strip_suffix('\r').unwrap_or(piece);
                let line_start = at;
                at += piece.len() + 1;
                // The empty piece after the file's last line break is not a line.
                if raw.is_empty() && pieces.peek().is_none() && self.src.ends_with('\n') {
                    break;
                }
                if !in_container(raw, &chain) {
                    break;
                }
                let l = strip(raw, &chain);
                let t = l.trim_start();
                if l.len() - t.len() < 4
                    && t.len() >= flen
                    && t.chars().take_while(|c| *c == fch).count() >= flen
                    && t.trim_end_matches(fch).trim().is_empty()
                {
                    closed = true;
                    break;
                }
                let cut = (l.bytes().take(fence_indent))
                    .take_while(|b| *b == b' ')
                    .count();
                lines.push((
                    line_start,
                    raw.len() - l.len() + cut,
                    line_start + raw.len(),
                ));
            }
        }
        (lines, closed)
    }
}
