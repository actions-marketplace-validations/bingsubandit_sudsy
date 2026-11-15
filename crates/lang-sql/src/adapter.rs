//! sqruff's segment tree as our lossless tree.
//!
//! sqruff lexes and parses a whole file into segments: raw leaves (whitespace, comments, words,
//! symbols) and nodes named by sqlfluff's segment types. The adapter flattens the leaves in
//! order, drops the zero-width ones (`Indent`, `Dedent`, `EndOfFile`: layout is Sudsy's), keeps
//! the nodes the grammar names as spans of non-trivia tokens, and rebuilds the tree with the
//! shared trivia attachment rule, because sqruff puts whitespace and comments wherever its
//! grammar matched them.
//!
//! sqruff panics on some inputs (an `unwrap` in a dialect), so the parse runs under
//! `catch_unwind`: a panic is a parse error and the file is one `Unknown` token, left unchanged.

use crate::generated::kind as k;
use core_syntax::trivia::attach;
use core_syntax::{Grammar, SyntaxKind, SyntaxNode, TreeBuilder, grammars};
use provider_api::{ParseError, ParseResult};
use sqruff_lib_core::dialects::Dialect;
use sqruff_lib_core::dialects::init::DialectKind;
use sqruff_lib_core::dialects::syntax::SyntaxKind as Sq;
use sqruff_lib_core::parser::segments::{ErasedSegment, Tables};
use sqruff_lib_core::parser::{IndentationConfig, Parser};
use sqruff_lib_dialects::kind_to_dialect;
use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::OnceLock;
use strum::IntoEnumIterator;

/// The most bytes of one file; sqruff parses 0.3 to 1 MB/s and holds about 650 bytes per input byte at its peak: this bounds one parse to 2 s and 350 MB.
pub const MAX_BYTES: usize = 512 << 10;

/// The dialects compiled in, as `dialect` option values.
pub const DIALECTS: [&str; 4] = ["ansi", "postgres", "mysql", "tsql"];

fn dialect_kind(name: &str) -> DialectKind {
    match name {
        "postgres" => DialectKind::Postgres,
        "mysql" => DialectKind::Mysql,
        "tsql" => DialectKind::Tsql,
        _ => DialectKind::Ansi,
    }
}

thread_local! {
    // sqruff's segments are `Rc`: a dialect is built once per thread, on its first file.
    static BUILT: RefCell<Vec<(DialectKind, Dialect)>> = const { RefCell::new(Vec::new()) };
}

pub fn parse(src: &str, dialect: &str) -> ParseResult {
    let leaves = if src.len() > MAX_BYTES {
        Err(format!("file over {} KB", MAX_BYTES >> 10))
    } else {
        catch_unwind(AssertUnwindSafe(|| {
            sqruff_parse(src, dialect_kind(dialect))
        }))
        .unwrap_or_else(|_| Err("the SQL parser failed on this file".into()))
    };
    match leaves {
        Ok(flat) => build(src, flat),
        Err(message) => whole_file(src, message),
    }
}

/// The file as one `Unknown` token, with the reason.
fn whole_file(src: &str, message: String) -> ParseResult {
    let mut builder = TreeBuilder::new();
    builder.start_node(k::FILE);
    if !src.is_empty() {
        builder.token(k::UNKNOWN, src);
    }
    builder.token(k::EOF, "");
    builder.finish_node();
    ParseResult {
        tree: SyntaxNode::new_root(builder.finish()),
        errors: vec![ParseError {
            start: 0,
            len: src.len() as u32,
            code: "syntax-error".into(),
            message,
        }],
    }
}

struct Leaf {
    kind: SyntaxKind,
    start: u32,
    end: u32,
}

/// A grammar node over the leaves `first..end`.
struct Node {
    kind: SyntaxKind,
    first: usize,
    end: usize,
}

/// What the walk over sqruff's tree collects.
struct Flat {
    leaves: Vec<Leaf>,
    nodes: Vec<Node>,
    errors: Vec<ParseError>,
}

fn sqruff_parse(src: &str, kind: DialectKind) -> Result<Flat, String> {
    let root = BUILT.with_borrow_mut(|built| {
        let at = match built.iter().position(|(k, _)| *k == kind) {
            Some(at) => at,
            None => {
                let dialect = kind_to_dialect(&kind, None).ok_or("dialect not compiled in")?;
                built.push((kind, dialect));
                built.len() - 1
            }
        };
        let dialect = &built[at].1;
        let tables = Tables::default();
        let (segments, lex_errors) = dialect.lexer().lex(&tables, src);
        let parser = Parser::new_with_limits(
            dialect,
            IndentationConfig::default(),
            sqruff_lib_core::parser::DEFAULT_MAX_PARSE_DEPTH,
            0,
        );
        let root = parser
            .parse(&tables, &segments)
            .map_err(|e| e.description)?;
        Ok::<_, String>((root, lex_errors.len()))
    })?;
    let (Some(root), _) = root else {
        return Ok(Flat {
            leaves: Vec::new(),
            nodes: Vec::new(),
            errors: Vec::new(),
        });
    };
    let mut walk = Walk {
        src,
        kinds: kind_table(grammars::SQL.get()),
        flat: Flat {
            leaves: Vec::new(),
            nodes: Vec::new(),
            errors: Vec::new(),
        },
        pos: 0,
    };
    walk.visit(&root);
    if walk.pos != src.len() {
        return Err("the SQL parser dropped text".into());
    }
    Ok(walk.flat)
}

/// Our node kind by sqruff's segment type: the type's name in Pascal case, or `Statement` for
/// any other `*_statement`; `None` is a type with no node (transparent).
fn kind_table(grammar: &'static Grammar) -> &'static [Option<SyntaxKind>] {
    static TABLE: OnceLock<Vec<Option<SyntaxKind>>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = vec![None; Sq::iter().count()];
        for sq in Sq::iter() {
            let name: &'static str = sq.into();
            let pascal: String = name
                .split('_')
                .flat_map(|w| {
                    w.chars()
                        .next()
                        .map(|c| c.to_ascii_uppercase())
                        .into_iter()
                        .chain(w.chars().skip(1))
                })
                .collect();
            let ours = grammar
                .kind_by_name(&pascal)
                .filter(|k| !k.is_token() && *k != k::FILE)
                .or_else(|| name.ends_with("_statement").then_some(k::STATEMENT));
            table[sq as usize] = ours;
        }
        table
    })
}

struct Walk<'a> {
    src: &'a str,
    kinds: &'static [Option<SyntaxKind>],
    flat: Flat,
    pos: usize,
}

impl Walk<'_> {
    fn visit(&mut self, seg: &ErasedSegment) {
        let children = seg.segments();
        let ty = seg.get_type();
        if children.is_empty() {
            self.leaf(seg, ty);
            return;
        }
        let first = self.flat.leaves.len();
        let ours = match ty {
            Sq::Unparsable => Some(SyntaxKind::ERROR),
            _ => self.kinds[ty as usize],
        };
        let start = self.pos;
        for child in children {
            self.visit(child);
        }
        let end = self.flat.leaves.len();
        if ty == Sq::Unparsable && self.pos > start {
            self.flat.errors.push(ParseError {
                start: start as u32,
                len: (self.pos - start) as u32,
                code: "syntax-error".into(),
                message: "syntax error".into(),
            });
        }
        if let Some(kind) = ours.filter(|_| end > first) {
            self.flat.nodes.push(Node { kind, first, end });
        }
    }

    fn leaf(&mut self, seg: &ErasedSegment, ty: Sq) {
        let raw = seg.raw();
        if raw.is_empty() || seg.is_meta() {
            return;
        }
        let start = self.pos;
        let end = start + raw.len();
        // A leaf that is not the text at its place would break the round trip.
        if self.src.get(start..end) != Some(raw.as_str()) {
            self.pos = usize::MAX / 2;
            return;
        }
        self.pos = end;
        let kind = token_kind(ty, raw);
        if kind == k::UNKNOWN {
            self.flat.errors.push(ParseError {
                start: start as u32,
                len: raw.len() as u32,
                code: "syntax-error".into(),
                message: "unexpected text".into(),
            });
        }
        // `pos` order is source order, so a node's leaves are consecutive.
        self.flat.leaves.push(Leaf {
            kind,
            start: start as u32,
            end: end as u32,
        });
    }
}

/// Our token kind for a sqruff leaf; a type this table does not know is classified by its text.
fn token_kind(ty: Sq, raw: &str) -> SyntaxKind {
    match ty {
        Sq::Whitespace => k::WHITESPACE,
        Sq::Newline => k::NEWLINE,
        Sq::InlineComment => k::LINE_COMMENT,
        Sq::BlockComment => k::BLOCK_COMMENT,
        Sq::Unlexable => k::UNKNOWN,
        Sq::Comma => k::COMMA,
        Sq::Dot => k::DOT,
        Sq::Semicolon => k::SEMICOLON,
        Sq::StatementTerminator if raw == ";" => k::SEMICOLON,
        Sq::StartBracket => k::L_PAREN,
        Sq::EndBracket => k::R_PAREN,
        Sq::Keyword => k::KEYWORD,
        Sq::QuotedIdentifier => k::QUOTED_IDENTIFIER,
        Sq::QuotedLiteral => k::STRING,
        Sq::NumericLiteral => k::NUMBER,
        _ => match raw.chars().next() {
            Some(c) if c.is_alphabetic() || c == '_' => k::IDENTIFIER,
            Some('\'') => k::STRING,
            Some('"' | '`') => k::QUOTED_IDENTIFIER,
            Some(c) if c.is_ascii_digit() => k::NUMBER,
            _ => k::OPERATOR,
        },
    }
}

/// Replays the leaves and nodes into a tree: each token takes its leading trivia, itself and its
/// trailing trivia (`core_syntax::trivia::attach`), inside the innermost nodes that contain it.
fn build(src: &str, flat: Flat) -> ParseResult {
    let Flat {
        mut leaves,
        nodes,
        errors,
    } = flat;
    let len = src.len() as u32;
    leaves.push(Leaf {
        kind: k::EOF,
        start: len,
        end: len,
    });
    let grammar = grammars::SQL.get();
    let text = |i: usize| &src[leaves[i].start as usize..leaves[i].end as usize];
    let attached = attach(leaves.len(), |i| grammar.is_trivia(leaves[i].kind), text);
    // The index among the real tokens of a leaf index.
    let mut real_at = vec![0usize; leaves.len() + 1];
    for (r, &i) in attached.real.iter().enumerate() {
        real_at[i] = r;
    }
    // A node covers the real tokens `first..=last`; one with no real token is dropped.
    let spans: Vec<(usize, usize, SyntaxKind)> = nodes
        .iter()
        .filter_map(|n| {
            let first = (n.first..n.end).find(|&i| !grammar.is_trivia(leaves[i].kind))?;
            let last = (n.first..n.end).rfind(|&i| !grammar.is_trivia(leaves[i].kind))?;
            Some((real_at[first], real_at[last], n.kind))
        })
        .collect();
    // `nodes` is in post-order; opening order is by first token, outer (later end) first.
    let mut order: Vec<usize> = (0..spans.len()).collect();
    order.sort_by_key(|&i| {
        (
            spans[i].0,
            std::cmp::Reverse(spans[i].1),
            std::cmp::Reverse(i),
        )
    });
    let mut builder = TreeBuilder::new();
    builder.start_node(k::FILE);
    let mut open: Vec<usize> = Vec::new();
    let mut next = order.iter().copied().peekable();
    for (r, &i) in attached.real.iter().enumerate() {
        while open.last().is_some_and(|&last| last < r) {
            open.pop();
            builder.finish_node();
        }
        // The Eof token belongs to the root, not to a node.
        if leaves[i].kind != k::EOF {
            while let Some(n) = next.next_if(|&n| spans[n].0 == r) {
                builder.start_node(spans[n].2);
                open.push(spans[n].1);
            }
        }
        let emit = |b: &mut TreeBuilder, i: usize| b.token(leaves[i].kind, text(i));
        for t in attached.leading[r].clone() {
            emit(&mut builder, t);
        }
        emit(&mut builder, i);
        for t in attached.trailing[r].clone() {
            emit(&mut builder, t);
        }
    }
    for _ in open {
        builder.finish_node();
    }
    builder.finish_node();
    ParseResult {
        tree: SyntaxNode::new_root(builder.finish()),
        errors,
    }
}
