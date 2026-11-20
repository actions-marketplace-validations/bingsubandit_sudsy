//! Types of the expression language, written as text in the language's tables:
//! `any`, `null`, `bool`, `number`, `string`, `array<T>`, `{a: T; b: T}` (these keys only) and
//! `{*: T}` (any key).
//!
//! concept: none (infrastructure: expression language)

use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum Ty {
    Any,
    Null,
    Bool,
    Num,
    Str,
    Arr(Box<Ty>),
    Obj(Vec<(String, Ty)>),
    Map(Box<Ty>),
}

impl fmt::Display for Ty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ty::Any => f.write_str("any"),
            Ty::Null => f.write_str("null"),
            Ty::Bool => f.write_str("bool"),
            Ty::Num => f.write_str("number"),
            Ty::Str => f.write_str("string"),
            Ty::Arr(t) => write!(f, "array<{t}>"),
            Ty::Map(t) => write!(f, "{{string => {t}}}"),
            Ty::Obj(props) => {
                f.write_str("{")?;
                for (i, (k, t)) in props.iter().enumerate() {
                    if i > 0 {
                        f.write_str("; ")?;
                    }
                    write!(f, "{k}: {t}")?;
                }
                f.write_str("}")
            }
        }
    }
}

impl Ty {
    /// Parses a type text; `a|b` alternatives are not types (see [`alternatives`]).
    pub fn parse(text: &str) -> Ty {
        let mut p = TyParser {
            text: text.as_bytes(),
            at: 0,
        };
        p.ty()
    }

    /// Can a value of this type be used where `to` is expected? Scalars convert to each other
    /// as strings; collections only to collections.
    pub fn assignable(&self, to: &Ty) -> bool {
        use Ty::*;
        match (self, to) {
            (Any, _) | (_, Any) => true,
            (Null | Bool | Num | Str, Null | Bool | Num | Str) => true,
            (Arr(a), Arr(b)) => a.assignable(b),
            (Obj(_) | Map(_), Obj(_) | Map(_)) => true,
            _ => false,
        }
    }

    /// The type of the property `name`: `None` when an object has no such key.
    pub fn prop(&self, name: &str) -> Option<Ty> {
        match self {
            Ty::Obj(props) => (props.iter())
                .find(|(k, _)| k.eq_ignore_ascii_case(name))
                .map(|(_, t)| t.clone()),
            Ty::Map(t) => Some((**t).clone()),
            _ => Some(Ty::Any),
        }
    }
}

/// The `|`-separated alternatives of a parameter type text.
pub fn alternatives(text: &str) -> Vec<Ty> {
    text.split('|').map(|t| Ty::parse(t.trim())).collect()
}

struct TyParser<'a> {
    text: &'a [u8],
    at: usize,
}

impl TyParser<'_> {
    fn skip(&mut self) {
        while self
            .text
            .get(self.at)
            .is_some_and(|c| c.is_ascii_whitespace())
        {
            self.at += 1;
        }
    }

    fn word(&mut self) -> String {
        self.skip();
        let start = self.at;
        while self
            .text
            .get(self.at)
            .is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'*'))
        {
            self.at += 1;
        }
        String::from_utf8_lossy(&self.text[start..self.at]).into_owned()
    }

    fn eat(&mut self, c: u8) -> bool {
        self.skip();
        if self.text.get(self.at) == Some(&c) {
            self.at += 1;
            true
        } else {
            false
        }
    }

    fn ty(&mut self) -> Ty {
        if self.eat(b'{') {
            let mut props = Vec::new();
            while !self.eat(b'}') && self.at < self.text.len() {
                let key = self.word();
                self.eat(b':');
                let value = self.ty();
                if key == "*" {
                    self.eat(b';');
                    self.eat(b'}');
                    return Ty::Map(Box::new(value));
                }
                props.push((key, value));
                self.eat(b';');
            }
            return Ty::Obj(props);
        }
        match self.word().as_str() {
            "null" => Ty::Null,
            "bool" => Ty::Bool,
            "number" => Ty::Num,
            "string" => Ty::Str,
            "array" => {
                self.eat(b'<');
                let t = self.ty();
                self.eat(b'>');
                Ty::Arr(Box::new(t))
            }
            _ => Ty::Any,
        }
    }
}
