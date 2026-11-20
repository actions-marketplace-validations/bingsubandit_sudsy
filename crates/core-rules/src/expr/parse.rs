//! Lexer and parser of the expression language: literals, names, `.prop`, `.*`, `[index]`, calls,
//! `!`, comparisons, `&&` and `||`. Offsets are relative to the expression text.
//!
//! concept: none (infrastructure: expression language)

#[derive(Clone, Debug, PartialEq)]
pub enum Lit {
    Null,
    Bool(bool),
    Num,
    Str(String),
}

#[derive(Clone, Debug)]
pub struct Node {
    pub kind: Kind,
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug)]
pub enum Kind {
    Lit(Lit),
    Var(String),
    Prop(Box<Node>, String),
    Star(Box<Node>),
    Index(Box<Node>, Box<Node>),
    Call(String, Vec<Node>),
    Not(Box<Node>),
    Cmp(&'static str, Box<Node>, Box<Node>),
    And(Box<Node>, Box<Node>),
    Or(Box<Node>, Box<Node>),
}

pub struct ParseError {
    pub at: usize,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Name(String),
    Num,
    Str(String),
    Punct(&'static str),
}

struct Parser<'a> {
    text: &'a str,
    toks: Vec<(Tok, usize, usize)>,
    pos: usize,
}

const PUNCT: &[&str] = &[
    "&&", "||", "==", "!=", "<=", ">=", "(", ")", "[", "]", ".", ",", "!", "<", ">", "*",
];

fn lex(text: &str) -> Result<Vec<(Tok, usize, usize)>, ParseError> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        let start = i;
        if c.is_ascii_whitespace() {
            i += 1;
        } else if c == b'\'' {
            let mut value = String::new();
            i += 1;
            loop {
                match b.get(i) {
                    None => {
                        return Err(ParseError {
                            at: start,
                            message: "string is not closed".into(),
                        });
                    }
                    Some(b'\'') if b.get(i + 1) == Some(&b'\'') => {
                        value.push('\'');
                        i += 2;
                    }
                    Some(b'\'') => break,
                    Some(_) => {
                        let ch = text[i..].chars().next().unwrap_or(' ');
                        value.push(ch);
                        i += ch.len_utf8();
                    }
                }
            }
            i += 1;
            out.push((Tok::Str(value), start, i));
        } else if c.is_ascii_digit() || (c == b'-' && b.get(i + 1).is_some_and(u8::is_ascii_digit))
        {
            i += 1;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || matches!(b[i], b'.' | b'_')) {
                i += 1;
            }
            out.push((Tok::Num, start, i));
        } else if c.is_ascii_alphabetic() || c == b'_' {
            while i < b.len() && (b[i].is_ascii_alphanumeric() || matches!(b[i], b'_' | b'-')) {
                i += 1;
            }
            out.push((Tok::Name(text[start..i].to_string()), start, i));
        } else if let Some(p) = PUNCT.iter().find(|p| text[i..].starts_with(**p)) {
            i += p.len();
            out.push((Tok::Punct(p), start, i));
        } else {
            let ch = text[i..].chars().next().unwrap_or(' ');
            return Err(ParseError {
                at: i,
                message: format!("unexpected character `{ch}` in expression"),
            });
        }
    }
    Ok(out)
}

/// Parses the whole text as one expression.
pub fn parse(text: &str) -> Result<Node, ParseError> {
    let toks = lex(text)?;
    if toks.is_empty() {
        return Err(ParseError {
            at: 0,
            message: "empty expression".into(),
        });
    }
    let mut p = Parser { text, toks, pos: 0 };
    let node = p.or()?;
    match p.toks.get(p.pos) {
        None => Ok(node),
        Some((_, s, _)) => Err(ParseError {
            at: *s,
            message: format!("unexpected `{}`", p.slice(p.pos)),
        }),
    }
}

impl Parser<'_> {
    fn slice(&self, i: usize) -> &str {
        let (_, s, e) = &self.toks[i];
        &self.text[*s..*e]
    }

    fn eof(&self) -> ParseError {
        ParseError {
            at: self.text.len(),
            message: "unexpected end of expression".into(),
        }
    }

    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos).map(|t| &t.0)
    }

    fn eat(&mut self, p: &str) -> bool {
        if matches!(self.peek(), Some(Tok::Punct(q)) if *q == p) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, p: &str) -> Result<usize, ParseError> {
        let end = self.toks.get(self.pos).map(|t| t.2);
        if self.eat(p) {
            return Ok(end.unwrap_or(0));
        }
        match self.toks.get(self.pos) {
            None => Err(self.eof()),
            Some((_, s, _)) => Err(ParseError {
                at: *s,
                message: format!("expected `{p}` but found `{}`", self.slice(self.pos)),
            }),
        }
    }

    fn bin(
        &mut self,
        ops: &[&'static str],
        next: fn(&mut Self) -> Result<Node, ParseError>,
    ) -> Result<Node, ParseError> {
        let mut left = next(self)?;
        'outer: loop {
            for op in ops {
                if self.eat(op) {
                    let right = next(self)?;
                    let (start, end) = (left.start, right.end);
                    let (l, r) = (Box::new(left), Box::new(right));
                    let kind = match *op {
                        "||" => Kind::Or(l, r),
                        "&&" => Kind::And(l, r),
                        _ => Kind::Cmp(op, l, r),
                    };
                    left = Node { kind, start, end };
                    continue 'outer;
                }
            }
            return Ok(left);
        }
    }

    fn or(&mut self) -> Result<Node, ParseError> {
        self.bin(&["||"], Self::and)
    }

    fn and(&mut self) -> Result<Node, ParseError> {
        self.bin(&["&&"], Self::cmp)
    }

    fn cmp(&mut self) -> Result<Node, ParseError> {
        self.bin(&["==", "!=", "<=", ">=", "<", ">"], Self::unary)
    }

    fn unary(&mut self) -> Result<Node, ParseError> {
        let start = self.toks.get(self.pos).map_or(self.text.len(), |t| t.1);
        if self.eat("!") {
            let inner = self.unary()?;
            let end = inner.end;
            return Ok(Node {
                kind: Kind::Not(Box::new(inner)),
                start,
                end,
            });
        }
        self.postfix()
    }

    fn postfix(&mut self) -> Result<Node, ParseError> {
        let mut node = self.primary()?;
        loop {
            if self.eat(".") {
                let Some((tok, s, e)) = self.toks.get(self.pos).cloned() else {
                    return Err(self.eof());
                };
                self.pos += 1;
                let kind = match tok {
                    Tok::Name(n) => Kind::Prop(Box::new(node.clone()), n),
                    Tok::Punct("*") => Kind::Star(Box::new(node.clone())),
                    Tok::Num => Kind::Prop(Box::new(node.clone()), self.text[s..e].to_string()),
                    _ => {
                        return Err(ParseError {
                            at: s,
                            message: format!(
                                "expected a property name after `.` but found `{}`",
                                &self.text[s..e]
                            ),
                        });
                    }
                };
                node = Node {
                    kind,
                    start: node.start,
                    end: e,
                };
            } else if self.eat("[") {
                let index = self.or()?;
                let end = self.expect("]")?;
                node = Node {
                    kind: Kind::Index(Box::new(node.clone()), Box::new(index)),
                    start: node.start,
                    end,
                };
            } else {
                return Ok(node);
            }
        }
    }

    fn primary(&mut self) -> Result<Node, ParseError> {
        let Some((tok, start, end)) = self.toks.get(self.pos).cloned() else {
            return Err(self.eof());
        };
        self.pos += 1;
        let lit = |l| {
            Ok(Node {
                kind: Kind::Lit(l),
                start,
                end,
            })
        };
        match tok {
            Tok::Num => lit(Lit::Num),
            Tok::Str(s) => lit(Lit::Str(s)),
            Tok::Punct("(") => {
                let inner = self.or()?;
                self.expect(")")?;
                Ok(inner)
            }
            Tok::Name(name) => {
                if self.peek() == Some(&Tok::Punct("(")) {
                    let paren = self.toks[self.pos].1;
                    self.pos += 1;
                    let mut args = Vec::new();
                    if !self.eat(")") {
                        loop {
                            args.push(self.or()?);
                            if self.eat(")") {
                                break;
                            }
                            if self.pos >= self.toks.len() {
                                return Err(ParseError {
                                    at: paren,
                                    message: "`(` is not closed".into(),
                                });
                            }
                            self.expect(",")?;
                        }
                    }
                    let end = self.toks[self.pos - 1].2;
                    return Ok(Node {
                        kind: Kind::Call(name, args),
                        start,
                        end,
                    });
                }
                match name.to_ascii_lowercase().as_str() {
                    "true" => lit(Lit::Bool(true)),
                    "false" => lit(Lit::Bool(false)),
                    "null" => lit(Lit::Null),
                    "nan" | "infinity" => lit(Lit::Num),
                    _ => Ok(Node {
                        kind: Kind::Var(name),
                        start,
                        end,
                    }),
                }
            }
            Tok::Punct(p) => Err(ParseError {
                at: start,
                message: format!("unexpected `{p}`"),
            }),
        }
    }
}
