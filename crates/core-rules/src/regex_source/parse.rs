//! The regex parser: a pattern as the items the checks read (characters, sets, classes with their
//! members, syntax characters) and the escapes it contains, read by the rules of a [`Flavor`].
//!
//! concept: regex-source-checks
//! what: Rules about the source of regular expressions in literals and call arguments, by flavor.
//! tools: eslint no-control-regex no-regex-spaces no-empty-character-class no-misleading-character-class, ruff regex checks
//! reads: grammar:regex*

use super::flavor::Flavor;

/// What one item of a pattern is.
#[derive(Debug, PartialEq, Eq)]
pub enum Kind {
    /// A character by its code point, written plainly or as an escape (`a`, `\x41`, `\-`).
    Char(u32),
    /// A set of characters outside a class: `\d`, `\p{L}`.
    Set,
    /// A character with a meaning in the syntax: `( ) | * + ? { } . ^ $`.
    Meta(char),
    Class(Class),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Item {
    pub start: usize,
    pub kind: Kind,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Class {
    pub negated: bool,
    pub members: Vec<Member>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Member {
    Char(u32),
    /// `a-z`; the end is none when it is a set (`a-\d`).
    Range(u32, Option<u32>),
    Set,
}

/// A backslash and the character after it, with where it stands.
#[derive(Debug, PartialEq, Eq)]
pub struct Esc {
    pub start: usize,
    pub ch: char,
    pub in_class: bool,
    /// Right after the `[` of its class.
    pub after_open: bool,
    /// Right before the `]` of its class.
    pub before_close: bool,
}

#[derive(Debug, Default)]
pub struct Regex {
    pub items: Vec<Item>,
    pub escapes: Vec<Esc>,
}

enum Atom {
    Char(u32),
    Set,
}

struct Parser<'a> {
    s: &'a str,
    i: usize,
    f: &'a Flavor,
    escapes: Vec<Esc>,
}

pub fn parse(pattern: &str, flavor: &Flavor) -> Regex {
    let mut p = Parser {
        s: pattern,
        i: 0,
        f: flavor,
        escapes: Vec::new(),
    };
    let mut items = Vec::new();
    while let Some(c) = p.peek() {
        let start = p.i;
        let kind = match c {
            '\\' => match p.escape(None) {
                Some(Atom::Char(cp)) => Kind::Char(cp),
                Some(Atom::Set) => Kind::Set,
                None => break,
            },
            '[' => {
                p.i += 1;
                Kind::Class(p.class(start))
            }
            '(' | ')' | '|' | '*' | '+' | '?' | '{' | '}' | '.' | '^' | '$' => {
                p.i += 1;
                Kind::Meta(c)
            }
            _ => {
                p.i += c.len_utf8();
                Kind::Char(c as u32)
            }
        };
        items.push(Item { start, kind });
    }
    Regex {
        items,
        escapes: p.escapes,
    }
}

impl Parser<'_> {
    fn peek(&self) -> Option<char> {
        self.s[self.i..].chars().next()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.s.get(self.i + offset..)?.chars().next()
    }

    /// The escape at the backslash `i`; `open` is the start of the class it is in. `None`: the
    /// backslash ends the pattern.
    fn escape(&mut self, open: Option<usize>) -> Option<Atom> {
        let start = self.i;
        let e = self.s[start + 1..].chars().next()?;
        self.i = start + 1 + e.len_utf8();
        self.escapes.push(Esc {
            start,
            ch: e,
            in_class: open.is_some(),
            after_open: open.is_some_and(|o| start == o + 1),
            before_close: self.peek() == Some(']'),
        });
        if let Some(&(_, digits)) = self.f.fixed.iter().find(|f| f.0 == e) {
            if self.f.braced.contains(e) && self.peek() == Some('{') {
                if let Some(cp) = self.braced() {
                    return Some(Atom::Char(cp));
                }
            } else if let Some(cp) = self.hex(digits) {
                return Some(Atom::Char(cp));
            }
            return Some(Atom::Char(e as u32));
        }
        if self.f.braced.contains(e) && self.peek() == Some('{') {
            return Some(Atom::Char(self.braced().unwrap_or(e as u32)));
        }
        if self.f.sets.contains(e) {
            return Some(Atom::Set);
        }
        if self.f.properties.contains(e) {
            if self.peek() == Some('{') {
                self.i += self.s[self.i..].find('}').map_or(0, |n| n + 1);
            } else if self.peek().is_some() {
                self.i += self.peek().map_or(0, char::len_utf8);
            }
            return Some(Atom::Set);
        }
        // A backspace inside a class, the letter itself elsewhere.
        Some(Atom::Char(if e == 'b' && open.is_some() {
            8
        } else {
            e as u32
        }))
    }

    /// `n` hex digits at the position.
    fn hex(&mut self, n: usize) -> Option<u32> {
        let digits = self.s.get(self.i..self.i + n)?;
        let cp = u32::from_str_radix(digits, 16)
            .ok()
            .filter(|_| digits.bytes().all(|b| b.is_ascii_hexdigit()))?;
        self.i += n;
        Some(cp)
    }

    /// `{hex}` at the position.
    fn braced(&mut self) -> Option<u32> {
        let end = self.s[self.i..].find('}')? + self.i;
        let cp = u32::from_str_radix(&self.s[self.i + 1..end], 16).ok()?;
        self.i = end + 1;
        Some(cp)
    }

    /// One member of a class; `None`: the pattern ends.
    fn atom(&mut self, open: usize) -> Option<Atom> {
        let c = self.peek()?;
        if c == '\\' {
            return self.escape(Some(open));
        }
        self.i += c.len_utf8();
        Some(Atom::Char(c as u32))
    }

    /// The class whose `[` is at `open` (the position is after it).
    fn class(&mut self, open: usize) -> Class {
        let mut class = Class::default();
        if self.peek() == Some('^') {
            class.negated = true;
            self.i += 1;
        }
        if !self.f.empty_class && self.peek() == Some(']') {
            self.i += 1;
            class.members.push(Member::Char(']' as u32));
        }
        while let Some(c) = self.peek() {
            if c == ']' {
                self.i += 1;
                break;
            }
            if c == '[' && self.f.nested {
                self.i += 1;
                self.class(self.i - 1);
                class.members.push(Member::Set);
                continue;
            }
            if c == '['
                && self.f.posix
                && self.peek_at(1) == Some(':')
                && let Some(end) = self.s[self.i..].find(":]")
            {
                self.i += end + 2;
                class.members.push(Member::Set);
                continue;
            }
            let Some(first) = self.atom(open) else { break };
            let Atom::Char(from) = first else {
                class.members.push(Member::Set);
                continue;
            };
            let range = self.peek() == Some('-') && self.peek_at(1).is_some_and(|n| n != ']');
            if !range {
                class.members.push(Member::Char(from));
                continue;
            }
            self.i += 1;
            let to = match self.atom(open) {
                Some(Atom::Char(to)) => Some(to),
                _ => None,
            };
            class.members.push(Member::Range(from, to));
        }
        class
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::regex_source::flavor::named;

    fn class_of(p: &str, flavor: &str) -> Class {
        let r = parse(p, named(flavor).unwrap());
        match r.items.into_iter().next().map(|i| i.kind) {
            Some(Kind::Class(c)) => c,
            other => panic!("not a class: {other:?}"),
        }
    }

    #[test]
    fn code_escapes() {
        let r = parse(r"\x1f\u{1F600}A", named("ecmascript").unwrap());
        let cps: Vec<_> = r
            .items
            .iter()
            .map(|i| match i.kind {
                Kind::Char(c) => c,
                _ => 0,
            })
            .collect();
        assert_eq!(cps, [0x1f, 0x1f600, 0x41]);
    }

    #[test]
    fn classes() {
        let c = class_of(r"[^a-z\d-]", "ecmascript");
        assert!(c.negated);
        assert_eq!(
            c.members,
            [
                Member::Range('a' as u32, Some('z' as u32)),
                Member::Set,
                Member::Char('-' as u32)
            ]
        );
        assert!(class_of("[]", "ecmascript").members.is_empty());
        assert_eq!(class_of("[]a]", "pcre").members.len(), 2);
        assert_eq!(class_of("[a[:alpha:]]", "pcre").members.len(), 2);
    }
}
