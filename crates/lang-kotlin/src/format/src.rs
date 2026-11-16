//! The file as its significant tokens and the trivia between them.

use crate::generated::kind as k;
use core_syntax::{SyntaxKind, SyntaxNode, SyntaxToken};

/// What lies between two significant tokens, apart from blanks.
pub(super) enum Piece {
    /// Line breaks (at least one).
    Nl(u32),
    /// A comment, as written.
    Comment(String),
    /// Blanks, as written.
    Ws(String),
}

pub(super) struct Gap {
    pub pieces: Vec<Piece>,
    /// The trivia as written, for gaps inside a string.
    pub raw: String,
    /// The blanks before the token on its line, as written (empty when it does not start one).
    pub indent: String,
}

pub(super) struct Source {
    pub toks: Vec<SyntaxToken>,
    /// `gaps[i]` is before `toks[i]`; the last one is before the end of the file.
    pub gaps: Vec<Gap>,
    /// The string template a token belongs to (the innermost one), by order of appearance.
    pub in_string: Vec<Option<u32>>,
    /// Start offsets of the tokens.
    pub starts: Vec<u32>,
}

impl Source {
    pub fn new(root: &SyntaxNode) -> Self {
        let mut toks = Vec::new();
        let mut gaps = Vec::new();
        let mut in_string = Vec::new();
        let mut starts = Vec::new();
        let mut strings: Vec<u32> = Vec::new();
        let mut next_string = 0;
        let mut pending = Gap {
            pieces: Vec::new(),
            raw: String::new(),
            indent: String::new(),
        };
        for event in root.preorder_with_tokens() {
            use rowan::WalkEvent::{Enter, Leave};
            match event {
                Enter(rowan::NodeOrToken::Node(n)) if n.kind() == k::STRING_TEMPLATE => {
                    strings.push(next_string);
                    next_string += 1;
                }
                Leave(rowan::NodeOrToken::Node(n)) if n.kind() == k::STRING_TEMPLATE => {
                    strings.pop();
                }
                Enter(rowan::NodeOrToken::Token(t)) => {
                    let kind = t.kind();
                    if is_trivia(kind) {
                        add_trivia(&mut pending, kind, t.text());
                    } else {
                        if kind != k::END_OF_FILE {
                            in_string.push(strings.last().copied());
                            starts.push(t.text_range().start().into());
                            toks.push(t);
                        }
                        gaps.push(std::mem::replace(
                            &mut pending,
                            Gap {
                                pieces: Vec::new(),
                                raw: String::new(),
                                indent: String::new(),
                            },
                        ));
                    }
                }
                _ => {}
            }
        }
        if gaps.len() == toks.len() {
            gaps.push(pending);
        }
        Source {
            toks,
            gaps,
            in_string,
            starts,
        }
    }

    /// Is the gap before token `i` inside a string template?
    pub fn gap_in_string(&self, i: usize) -> bool {
        i > 0
            && i < self.toks.len()
            && self.in_string[i].is_some()
            && self.in_string[i] == self.in_string[i - 1]
    }

    /// The index of the first token starting at or after `offset`.
    pub fn index_at(&self, offset: u32) -> usize {
        self.starts.partition_point(|&s| s < offset)
    }
}

fn is_trivia(kind: SyntaxKind) -> bool {
    kind == k::WHITESPACE || kind == k::NEWLINE || kind == k::COMMENT
}

fn add_trivia(gap: &mut Gap, kind: SyntaxKind, text: &str) {
    gap.raw.push_str(text);
    gap.indent = if kind == k::WHITESPACE && matches!(gap.pieces.last(), Some(Piece::Nl(_))) {
        text.to_string()
    } else if kind == k::WHITESPACE {
        gap.indent.clone() + text
    } else {
        String::new()
    };
    if kind == k::NEWLINE {
        let n = text.matches('\n').count().max(1) as u32;
        match gap.pieces.last_mut() {
            Some(Piece::Nl(c)) => *c += n,
            Some(Piece::Ws(_)) => {
                gap.pieces.pop();
                add_trivia(gap, kind, text);
            }
            _ => gap.pieces.push(Piece::Nl(n)),
        }
    } else if kind == k::COMMENT {
        gap.pieces.push(Piece::Comment(text.replace("\r\n", "\n")));
    } else if let Some(Piece::Ws(w)) = gap.pieces.last_mut() {
        w.push_str(text);
    } else {
        gap.pieces.push(Piece::Ws(text.to_string()));
    }
}
