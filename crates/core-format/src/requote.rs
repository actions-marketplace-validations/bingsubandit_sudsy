//! The quote of a markup attribute value: the one that needs fewer escapes (double when equal),
//! the other one escaped as the language's entity. A value may hold pieces that cannot be
//! escaped (template actions): a quote inside one counts a thousand times, so the other quote
//! delimits when it can, and only the text outside the pieces is escaped.
//!
//! concept: attribute-requote
//! what: Quote choice and escaping of an attribute value, with protected pieces.
//! tools: prettier-html
//! reads: path:requote::Entities

use std::borrow::Cow;

/// The entities that stand for each quote in a value.
pub struct Entities {
    pub single: &'static str,
    pub double: &'static str,
}

impl Entities {
    /// `text` with the entities of the quotes read back as the quotes.
    pub fn plain(&self, text: &str) -> String {
        text.replace(self.single, "'").replace(self.double, "\"")
    }
}

/// `value`'s pieces are `(protected, text)`; a text that already holds an entity for a quote has
/// it read back as the quote first. Returns the delimiter and the value with the delimiter
/// escaped.
pub fn requote<'a>(pieces: &[(bool, &'a str)], ent: &Entities) -> (char, Cow<'a, str>) {
    // A single text with no quote and no entity stays as it is, double-quoted.
    if let [(_, t)] = pieces
        && !t.contains(['\'', '"'])
        && !t.contains(ent.single)
        && !t.contains(ent.double)
    {
        return ('"', Cow::Borrowed(t));
    }
    let plain = |t: &str| ent.plain(t);
    let cost = |c: char| {
        pieces
            .iter()
            .map(|&(protected, t)| {
                let n = plain(t).matches(c).count();
                if protected { n * 1000 } else { n }
            })
            .sum::<usize>()
    };
    let (q, esc) = if cost('\'') < cost('"') {
        ('\'', ent.single)
    } else {
        ('"', ent.double)
    };
    let value = pieces
        .iter()
        .map(|&(protected, t)| {
            if protected {
                t.to_string()
            } else {
                plain(t).replace(q, esc)
            }
        })
        .collect::<String>();
    (q, Cow::Owned(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HTML: Entities = Entities {
        single: "&apos;",
        double: "&quot;",
    };

    #[test]
    fn fewer_escapes_win() {
        assert_eq!(requote(&[(false, "a\"b")], &HTML), ('\'', "a\"b".into()));
        assert_eq!(requote(&[(false, "a'b")], &HTML), ('"', "a'b".into()));
        assert_eq!(requote(&[(false, "&quot;x")], &HTML), ('\'', "\"x".into()));
    }

    #[test]
    fn protected_quotes_stay() {
        let p = [(true, "{{ \"x\" }}"), (false, "'")];
        assert_eq!(requote(&p, &HTML), ('\'', "{{ \"x\" }}&apos;".into()));
    }
}
