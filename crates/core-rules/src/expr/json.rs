//! The type of a JSON text, for functions that turn text into a value (`fromJSON('{"a": 1}')`).
//!
//! concept: none (infrastructure: expression language)

use super::ty::Ty;

/// The type of the JSON in `text`, or the offset and reason it is broken.
pub fn parse(text: &str) -> Result<Ty, (usize, &'static str)> {
    let mut p = Json {
        b: text.as_bytes(),
        at: 0,
    };
    p.skip();
    let t = p.value()?;
    p.skip();
    if p.at < p.b.len() {
        return Err((p.at, "invalid character after top-level value"));
    }
    Ok(t)
}

struct Json<'a> {
    b: &'a [u8],
    at: usize,
}

const EOF: &str = "unexpected end of JSON input";

impl Json<'_> {
    fn skip(&mut self) {
        while self.b.get(self.at).is_some_and(u8::is_ascii_whitespace) {
            self.at += 1;
        }
    }

    fn eat(&mut self, c: u8) -> bool {
        if self.b.get(self.at) == Some(&c) {
            self.at += 1;
            true
        } else {
            false
        }
    }

    fn word(&mut self, w: &str, t: Ty) -> Result<Ty, (usize, &'static str)> {
        if self.b[self.at..].starts_with(w.as_bytes()) {
            self.at += w.len();
            Ok(t)
        } else if w.as_bytes().starts_with(&self.b[self.at..]) {
            Err((self.b.len(), EOF))
        } else {
            Err((self.at, "invalid character"))
        }
    }

    fn string(&mut self) -> Result<(), (usize, &'static str)> {
        self.at += 1;
        loop {
            match self.b.get(self.at) {
                None => return Err((self.b.len(), EOF)),
                Some(b'"') => {
                    self.at += 1;
                    return Ok(());
                }
                Some(b'\\') => self.at += 2,
                Some(_) => self.at += 1,
            }
        }
    }

    fn value(&mut self) -> Result<Ty, (usize, &'static str)> {
        match self.b.get(self.at) {
            None => Err((self.b.len(), EOF)),
            Some(b'"') => self.string().map(|()| Ty::Str),
            Some(b't') => self.word("true", Ty::Bool),
            Some(b'f') => self.word("false", Ty::Bool),
            Some(b'n') => self.word("null", Ty::Null),
            Some(b'[') => {
                self.at += 1;
                self.skip();
                let mut items = Vec::new();
                if !self.eat(b']') {
                    loop {
                        self.skip();
                        items.push(self.value()?);
                        self.skip();
                        if self.eat(b']') {
                            break;
                        }
                        if !self.eat(b',') {
                            return Err(self.expected());
                        }
                    }
                }
                let first = items.first().cloned().unwrap_or(Ty::Any);
                let same = items.iter().all(|t| *t == first);
                Ok(Ty::Arr(Box::new(if same { first } else { Ty::Any })))
            }
            Some(b'{') => {
                self.at += 1;
                self.skip();
                let mut props = Vec::new();
                if !self.eat(b'}') {
                    loop {
                        self.skip();
                        if self.b.get(self.at) != Some(&b'"') {
                            return Err(self.expected());
                        }
                        let from = self.at + 1;
                        self.string()?;
                        let key = String::from_utf8_lossy(&self.b[from..self.at - 1]).into_owned();
                        self.skip();
                        if !self.eat(b':') {
                            return Err(self.expected());
                        }
                        self.skip();
                        props.push((key, self.value()?));
                        self.skip();
                        if self.eat(b'}') {
                            break;
                        }
                        if !self.eat(b',') {
                            return Err(self.expected());
                        }
                    }
                }
                Ok(Ty::Obj(props))
            }
            Some(c) if c.is_ascii_digit() || *c == b'-' => {
                let from = self.at;
                self.at += 1;
                while self.b.get(self.at).is_some_and(|c| {
                    c.is_ascii_digit() || matches!(c, b'.' | b'e' | b'E' | b'+' | b'-')
                }) {
                    self.at += 1;
                }
                let n = std::str::from_utf8(&self.b[from..self.at]).unwrap_or("");
                if n.parse::<f64>().is_ok() {
                    Ok(Ty::Num)
                } else {
                    Err((from, "invalid number"))
                }
            }
            Some(_) => Err((self.at, "invalid character")),
        }
    }

    fn expected(&self) -> (usize, &'static str) {
        if self.at >= self.b.len() {
            (self.b.len(), EOF)
        } else {
            (self.at, "invalid character")
        }
    }
}
