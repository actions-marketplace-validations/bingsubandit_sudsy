//! Angular: control flow blocks (`@if (a) {` ... `}`), `@let`, ICU expressions
//! (`{n, plural, =0 {none}}`) and `{{ }}` closed outside string literals, as its compiler reads them.
//!
//! concept: recursive-descent-parser

use super::{Open, P, is_ws};
use crate::generated::kind as k;

/// The first words of the control flow blocks (`@else if` and `@default never` start with these).
const BLOCKS: [&str; 13] = [
    "if",
    "else",
    "for",
    "empty",
    "switch",
    "case",
    "default",
    "defer",
    "placeholder",
    "loading",
    "error",
    "boundary",
    "content",
];

/// One step through expression text: strings open at `'`, `"` and `` ` ``, a `\` escapes the next
/// byte. Returns the bytes taken and whether `b` is outside any string.
fn string_step(b: u8, quote: &mut Option<u8>) -> (usize, bool) {
    match (b, *quote) {
        (b'\\', _) => (2, false),
        (b, Some(q)) if b == q => {
            *quote = None;
            (1, false)
        }
        (_, Some(_)) => (1, false),
        (q @ (b'\'' | b'"' | b'`'), None) => {
            *quote = Some(q);
            (1, false)
        }
        _ => (1, true),
    }
}

impl P<'_> {
    /// In an ICU expression or one of its cases: blocks and `@let` do not start there.
    fn in_icu(&self) -> bool {
        let top = self
            .stack
            .iter()
            .rev()
            .find(|(o, _)| !matches!(o, Open::Element(..)));
        matches!(top, Some((Open::Icu | Open::IcuCase, _)))
    }

    /// `@let`, or `@` and a block name, at `i`.
    pub(super) fn ng_candidate(&self, i: usize) -> bool {
        let rest = &self.src[i + 1..];
        !self.in_icu() && (rest.starts_with("let") || BLOCKS.iter().any(|b| rest.starts_with(b)))
    }

    /// A block or `@let` at `pos`; one that is incomplete is an error and the `@` is text.
    pub(super) fn ng_at(&mut self) -> bool {
        if !self.ng_candidate(self.pos) {
            return false;
        }
        let ok = if self.src[self.pos..].starts_with("@let") {
            self.ng_let()
        } else {
            self.ng_block()
        };
        if !ok {
            self.text(true);
        }
        true
    }

    /// The `}}` that closes an interpolation whose expression starts at `from`: outside strings,
    /// and before any tag.
    pub(super) fn ng_interpolation_end(&self, from: usize) -> Option<usize> {
        let (mut quote, mut comment) = (None, false);
        let mut at = from;
        while at < self.b.len() {
            if self.b[at] == b'<' && self.tag_start(at) && self.at(at + 1) != b'?' {
                return None;
            }
            if quote.is_none() {
                if self.src[at..].starts_with("}}") {
                    return Some(at);
                }
                if self.src[at..].starts_with("//") {
                    comment = true;
                    at += 2;
                }
            }
            let Some(&b) = self.b.get(at) else { break };
            let (n, _) = if comment {
                (if b == b'\\' { 2 } else { 1 }, false)
            } else {
                string_step(b, &mut quote)
            };
            at += n;
        }
        None
    }

    fn trimmed(&self, mut s: usize, mut e: usize) -> (usize, usize) {
        while s < e && is_ws(self.b[s]) {
            s += 1;
        }
        while e > s && is_ws(self.b[e - 1]) {
            e -= 1;
        }
        (s, e)
    }

    /// The punctuation of the syntax in the text `a..b`; blanks are trivia.
    fn punct(&mut self, a: usize, b: usize) {
        for (i, c) in self.src[a..b].char_indices() {
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
            self.tok(kind, a + i, a + i + 1);
        }
    }

    /// `@name (parameters) {` and its content; false when the head is incomplete.
    fn ng_block(&mut self) -> bool {
        let start = self.pos;
        let (mut at, mut name_end) = (start + 1, start + 1);
        while self.at(at).is_ascii_alphanumeric() || self.at(at) == b'_' || is_ws(self.at(at)) {
            if !is_ws(self.at(at)) {
                name_end = at + 1;
            }
            at += 1;
        }
        let words: Vec<&str> = self.src[start + 1..name_end].split_whitespace().collect();
        let first = words.first().copied().unwrap_or("");
        let mut params = Vec::new();
        let mut open_end = name_end;
        if self.at(at) == b'(' {
            let Some(end) = self.ng_parameters(at + 1, &mut params) else {
                self.error(start, name_end, "block is not complete");
                return false;
            };
            open_end = end;
            at = self.skip_ws(end);
        }
        let body = self.at(at) == b'{';
        let never = words == ["default", "never"] && self.at(at) == b';';
        if body || never {
            open_end = at + 1;
        } else if !(matches!(first, "case" | "default")
            && self.at(at) == b'@'
            && self.ng_candidate(at))
        {
            self.error(start, open_end, "block is not complete");
            return false;
        }
        // A `;` after the last parameter belongs to it.
        if let Some(last) = params.last_mut() {
            let rest = &self.src[last.1..open_end];
            if rest.trim_start().starts_with(';') {
                last.1 += rest.find(';').unwrap_or(0) + 1;
            }
        }
        let block = self.open(k::NG_BLOCK, start);
        let head = self.open(k::BLOCK_HEAD, start);
        self.tok(k::AT_NAME, start, name_end);
        let mut cursor = name_end;
        for (s, e) in params {
            self.punct(cursor, s);
            self.tok(k::NG_CODE, s, e);
            cursor = e;
        }
        self.punct(cursor, open_end);
        self.close(head);
        self.pos = open_end;
        if body {
            self.stack.push((Open::Block, block));
        } else {
            self.close(block);
        }
        true
    }

    /// The parameters after `(`, split at `;` outside strings and parentheses; the offset after the `)`.
    fn ng_parameters(&self, from: usize, out: &mut Vec<(usize, usize)>) -> Option<usize> {
        let (mut quote, mut depth, mut param) = (None, 0usize, from);
        let mut at = from;
        let push = |s: usize, e: usize, out: &mut Vec<(usize, usize)>| {
            let t = self.trimmed(s, e);
            if t.0 < t.1 {
                out.push(t);
            }
        };
        while at < self.b.len() {
            let b = self.b[at];
            let (n, outside) = string_step(b, &mut quote);
            if outside {
                match b {
                    b'(' => depth += 1,
                    b')' if depth > 0 => depth -= 1,
                    b')' => {
                        push(param, at, out);
                        return Some(at + 1);
                    }
                    b';' => {
                        push(param, at, out);
                        param = at + 1;
                        depth = 0;
                    }
                    _ => {}
                }
            }
            at += n;
        }
        None
    }

    /// `@let name = value;`; false when it is incomplete.
    fn ng_let(&mut self) -> bool {
        let start = self.pos;
        let name_start = self.skip_ws(start + 4);
        let incomplete = |p: &mut Self, end: usize| {
            p.error(start, end.max(start + 1), "@let is not complete");
            false
        };
        if name_start == start + 4 {
            return incomplete(self, name_start);
        }
        let mut name_end = name_start;
        if !self.at(name_start).is_ascii_digit() {
            while self.at(name_end).is_ascii_alphanumeric()
                || matches!(self.at(name_end), b'_' | b'$')
            {
                name_end += 1;
            }
        }
        let eq = self.skip_ws(name_end);
        if self.at(eq) != b'=' {
            return incomplete(self, eq);
        }
        let value_start = self.skip_ws(eq + 1);
        let (mut quote, mut at) = (None, value_start);
        let semi = loop {
            let Some(&b) = self.b.get(at) else {
                return incomplete(self, self.b.len());
            };
            match b {
                b';' if quote.is_none() => break at,
                b'\\' if quote.is_none() => at += 2,
                _ => at += string_step(b, &mut quote).0,
            }
        };
        let (vs, ve) = self.trimmed(value_start, semi);
        let n = self.open(k::NG_LET, start);
        self.tok(k::AT_NAME, start, start + 4);
        self.punct(start + 4, name_start);
        self.tok(k::NG_CODE, name_start, name_end);
        self.punct(name_end, vs);
        self.tok(k::NG_CODE, vs, ve);
        self.punct(ve, semi + 1);
        self.close(n);
        self.pos = semi + 1;
        true
    }

    /// `{`: an interpolation (`{{`) or the start of an ICU expression.
    pub(super) fn ng_brace(&mut self) {
        if self.at(self.pos + 1) == b'{' {
            if !self.interpolation() {
                self.text_from(2);
            }
        } else if !self.ng_icu_start() {
            self.text(true);
        }
    }

    /// `{expression, type,` and the first case.
    fn ng_icu_start(&mut self) -> bool {
        let start = self.pos;
        let comma = |from: usize| {
            let rest = &self.src[from..];
            let stop = rest.find(['{', '}', '<']).unwrap_or(rest.len());
            rest[..stop].find(',').map(|i| from + i)
        };
        let Some(c1) = comma(start + 1) else {
            self.error(start, start + 1, "invalid ICU message");
            return false;
        };
        let type_start = self.skip_ws(c1 + 1);
        let Some(c2) = comma(type_start) else {
            self.error(start, start + 1, "invalid ICU message");
            return false;
        };
        let n = self.open(k::ICU, start);
        self.tok(k::L_BRACE, start, start + 1);
        let (es, ee) = self.trimmed(start + 1, c1);
        self.tok(k::NG_CODE, es, ee);
        self.tok(k::COMMA, c1, c1 + 1);
        let (ts, te) = self.trimmed(type_start, c2);
        self.tok(k::NG_CODE, ts, te);
        self.tok(k::COMMA, c2, c2 + 1);
        self.stack.push((Open::Icu, n));
        self.pos = c2 + 1;
        self.ng_icu_next();
        true
    }

    /// After the head of an ICU expression or one of its cases: the next case up to its `{`, or the
    /// `}` that closes the expression.
    fn ng_icu_next(&mut self) {
        let at = self.skip_ws(self.pos);
        let open = if self.at(at) == b'}' {
            None
        } else {
            self.src[at..].find('{').map(|i| at + i)
        };
        let Some(open) = open else {
            if self.at(at) == b'}' {
                self.tok(k::R_BRACE, at, at + 1);
                self.pos = at + 1;
            } else {
                self.error(at, at + 1, "invalid ICU message");
                self.pos = self.b.len();
            }
            let (_, node) = self.stack.pop().expect("an ICU expression is open");
            self.close(node);
            return;
        };
        let n = self.open(k::ICU_CASE, at);
        let (vs, ve) = self.trimmed(at, open);
        self.tok(k::NG_CODE, vs, ve);
        self.tok(k::L_BRACE, open, open + 1);
        self.stack.push((Open::IcuCase, n));
        self.pos = open + 1;
    }

    /// `}`: closes the innermost block or ICU case, and the elements left open in it.
    pub(super) fn ng_close(&mut self) {
        let at = self.pos;
        let Some(i) = self
            .stack
            .iter()
            .rposition(|(o, _)| matches!(o, Open::Block | Open::IcuCase))
        else {
            self.error(at, at + 1, "unexpected }");
            return self.text(true);
        };
        while self.stack.len() > i + 1 {
            let (_, node) = self.stack.pop().expect("an element is open");
            self.close(node);
        }
        self.tok(k::R_BRACE, at, at + 1);
        self.pos = at + 1;
        let (open, node) = self.stack.pop().expect("a container is open");
        self.close(node);
        if open == Open::IcuCase {
            self.ng_icu_next();
        }
    }
}
