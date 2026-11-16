//! Angular templates: `oxc-htmlish-parser`'s `Dialect::Angular` (the reading of Prettier's
//! `angular-html-parser` and of Angular's compiler) as our HTML tree. The parser gives spans only;
//! this file turns each node into the tokens and nodes of `grammars/html.grammar` and
//! `provider_api::adapter::replay` builds the tree. A file the parser rejects is one error token with
//! the parser's message: there is no tree to recover.

use crate::generated::kind as k;
use core_syntax::SyntaxKind;
use oxc_htmlish_parser::ast::{
    AttributeKind, BlockParameter, Content, Element, Node as HNode, NodeKind, Root,
};
use oxc_htmlish_parser::{Allocator, Dialect, Parser, Span};
use provider_api::adapter::{Kinds, Node, Tok, replay};
use provider_api::{ParseError, ParseResult, SourceFile};

const KINDS: Kinds = Kinds {
    root: k::DOCUMENT,
    eof: k::EOF,
    unknown: k::UNKNOWN,
    whitespace: k::WHITESPACE,
    newline: k::NEWLINE,
};

pub fn parse_file(file: &SourceFile) -> ParseResult {
    let src = file.text.as_str();
    let allocator = Allocator::default();
    match Parser::new(&allocator, src, Dialect::Angular).parse() {
        Ok(root) => {
            let mut w = Walk {
                src,
                root: &root,
                tokens: Vec::new(),
                nodes: Vec::new(),
            };
            w.children(oxc_htmlish_parser::ast::NodeId::DOCUMENT, false);
            replay(src, KINDS, &w.tokens, &w.nodes, scan, |t, _| t)
        }
        Err(d) => {
            let mut parsed = replay(src, KINDS, &[], &[], scan, |t, _| t);
            parsed.errors = vec![ParseError::syntax(
                d.span.start,
                d.span.end - d.span.start,
                d.to_string(),
            )];
            parsed
        }
    }
}

/// Comments sit between nodes: `<!-- ... -->` (conditional comments included) and `<!x>`, `<?x>`.
fn scan(rest: &str, _pos: usize) -> Option<(SyntaxKind, usize)> {
    let end = if rest.starts_with("<!--") {
        rest.find("-->").map(|i| i + 3)
    } else if rest.starts_with("<!") || rest.starts_with("<?") {
        rest.find('>').map(|i| i + 1)
    } else {
        None
    };
    end.map(|len| (k::COMMENT, len))
}

struct Walk<'a, 'r> {
    src: &'a str,
    root: &'r Root<'a>,
    tokens: Vec<Tok>,
    nodes: Vec<Node>,
}

impl Walk<'_, '_> {
    fn token(&mut self, kind: SyntaxKind, s: Span) {
        if s.start < s.end {
            self.tokens.push(Tok {
                kind,
                start: s.start,
                end: s.end,
            });
        }
    }

    fn node(&mut self, kind: SyntaxKind, s: Span) {
        self.nodes.push(Node {
            start: s.start,
            end: s.end,
            kind,
        });
    }

    /// The punctuation of the Angular syntax in the text `a..b`; blanks and the rest are the
    /// replay's.
    fn punct(&mut self, a: u32, b: u32) {
        for (i, c) in self.src[a as usize..b as usize].char_indices() {
            let kind = match c {
                '=' => k::EQUALS,
                '(' => k::L_PAREN,
                ')' => k::R_PAREN,
                ';' => k::SEMI,
                ',' => k::COMMA,
                '{' => k::L_BRACE,
                '}' => k::R_BRACE,
                _ => continue,
            };
            let at = a + i as u32;
            self.token(kind, Span::new(at, at + 1));
        }
    }

    /// Tokens of the spans `parts` (in order, kind `kind`) with the punctuation between them,
    /// from `from` to `to`.
    fn parts(&mut self, kind: SyntaxKind, from: u32, to: u32, parts: &[Span]) {
        let mut at = from;
        for p in parts {
            self.punct(at, p.start);
            self.token(kind, *p);
            at = p.end;
        }
        self.punct(at, to);
    }

    fn children(&mut self, id: oxc_htmlish_parser::ast::NodeId, raw: bool) {
        let ids: Vec<_> = self.root.children(id).map(|(i, _)| i).collect();
        for i in ids {
            self.visit(i, raw);
        }
    }

    fn visit(&mut self, id: oxc_htmlish_parser::ast::NodeId, raw: bool) {
        let root = self.root;
        let n: &HNode = &root[id];
        match &n.kind {
            NodeKind::Text(t) => {
                if !t.content.is_empty() {
                    self.node(k::CHUNK, t.content);
                    self.token(if raw { k::RAW_TEXT } else { k::TEXT }, t.content);
                }
            }
            NodeKind::Interpolation(i) => {
                let e = i.expression;
                self.node(k::INTERPOLATION, Span::new(e.start - 2, e.end + 2));
                self.token(k::L_BRACE_BRACE, Span::new(e.start - 2, e.start));
                if !e.is_empty() {
                    self.node(k::CHUNK, e);
                    self.token(k::TEXT, e);
                }
                self.token(k::R_BRACE_BRACE, Span::new(e.end, e.end + 2));
            }
            NodeKind::Cdata => {
                self.node(k::CHUNK, n.span);
                self.token(k::C_DATA, n.span);
            }
            NodeKind::DocType(d) => {
                let s = n.span;
                self.node(k::DOCTYPE, s);
                self.token(k::LT_BANG, Span::new(s.start, s.start + 2));
                let word = s.start + 2;
                self.token(k::DOCTYPE_KW, Span::new(word, word + 7));
                self.token(k::DOCTYPE_BODY, d.value);
                self.token(k::GT, Span::new(s.end - 1, s.end));
            }
            NodeKind::Element(e) => self.element(id, n.span, e),
            NodeKind::EndTag(e) => {
                self.node(k::STRAY_END_TAG, e.span);
                self.end_tag(e.span, e.name);
            }
            NodeKind::Block(b) => {
                self.node(k::NG_BLOCK, n.span);
                self.node(k::BLOCK_HEAD, b.open);
                self.token(k::AT_NAME, Span::new(b.open.start, b.name.end));
                let mut params: Vec<Span> = b
                    .parameters
                    .iter()
                    .map(|p: &BlockParameter| p.span)
                    .collect();
                // A `;` after the last parameter belongs to it: the printer drops it.
                if let Some(last) = params.last_mut() {
                    let rest = &self.src[last.end as usize..b.open.end as usize];
                    if rest.trim_start().starts_with(';') {
                        last.end += rest.find(';').unwrap() as u32 + 1;
                    }
                }
                self.parts(k::NG_CODE, b.name.end, b.open.end, &params);
                self.children(id, false);
                if let Some(c) = b.close {
                    self.token(k::R_BRACE, c);
                }
            }
            NodeKind::LetDeclaration(l) => {
                self.node(k::NG_LET, n.span);
                self.token(k::AT_NAME, Span::new(n.span.start, n.span.start + 4));
                self.parts(k::NG_CODE, n.span.start + 4, n.span.end, &[l.name, l.value]);
            }
            NodeKind::Icu(i) => {
                self.node(k::ICU, n.span);
                self.token(k::L_BRACE, Span::new(n.span.start, n.span.start + 1));
                let first = self
                    .root
                    .children(id)
                    .next()
                    .map_or(n.span.end - 1, |(_, c)| c.span.start);
                self.parts(
                    k::NG_CODE,
                    n.span.start + 1,
                    first,
                    &[i.expression, i.type_name],
                );
                self.children(id, false);
                self.token(k::R_BRACE, Span::new(n.span.end - 1, n.span.end));
            }
            NodeKind::IcuCase(c) => {
                self.node(k::ICU_CASE, n.span);
                self.token(k::NG_CODE, c.value);
                self.token(k::L_BRACE, Span::new(c.open, c.open + 1));
                self.children(id, false);
                self.token(k::R_BRACE, Span::new(n.span.end - 1, n.span.end));
            }
            // Comments are the replay's (`scan`); the content of a conditional comment is part of it.
            _ => {}
        }
    }

    fn element(&mut self, id: oxc_htmlish_parser::ast::NodeId, span: Span, e: &Element<'_>) {
        self.node(k::ELEMENT, span);
        let st = e.start_tag;
        self.node(k::START_TAG, st);
        self.token(k::LT, Span::new(st.start, st.start + 1));
        self.token(k::TAG_NAME, e.name);
        for a in e.attributes.iter() {
            if matches!(a.kind, AttributeKind::Comment) {
                continue;
            }
            self.node(k::ATTRIBUTE, a.span());
            self.token(k::ATTR_NAME, a.name);
            if let Some(v) = a.value {
                let quoted = u32::from(v.quote != oxc_htmlish_parser::ast::Quote::None);
                let value = Span::new(v.span.start - quoted, v.span.end + quoted);
                self.punct(a.name.end, value.start);
                self.token(k::ATTR_VALUE, value);
            }
        }
        if e.self_closing && self.src[..st.end as usize].ends_with("/>") {
            self.token(k::SLASH_GT, Span::new(st.end - 2, st.end));
        } else if self.src[..st.end as usize].ends_with('>') {
            self.token(k::GT, Span::new(st.end - 1, st.end));
        }
        let raw = e.content != Content::Normal && e.content != Content::Escapable;
        self.children(id, raw);
        if let Some(t) = e.end_tag {
            self.node(k::END_TAG, t.span);
            self.end_tag(t.span, t.name);
        }
    }

    fn end_tag(&mut self, span: Span, name: Span) {
        self.token(k::LT_SLASH, Span::new(span.start, span.start + 2));
        self.token(k::TAG_NAME, name);
        if self.src[..span.end as usize].ends_with('>') {
            self.token(k::GT, Span::new(span.end - 1, span.end));
        }
    }
}
