//! Prefixed strings (`r".."`, `f".."`, `"""..""""`): quote choice by escapes, prefix spelling and
//! f-string fields (the rule of Black and ruff).
//!
//! concept: prefixed-string-quotes
//! what: Quote choice, prefix spelling and f-string fields of prefixed strings.
//! tools: Black string prefix normalization, ruff format

use core_syntax::strings::StringSyntax;
use std::borrow::Cow;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Quote {
    Double,
    Single,
    Preserve,
}

/// A string token's pieces: prefix, quote (1 or 3 characters) and body.
pub fn split<'a>(syntax: &StringSyntax, text: &'a str) -> Option<(&'a str, &'a str, &'a str)> {
    let lit = syntax.split(text)?;
    Some((lit.prefix, lit.open, lit.inner))
}

/// The prefix as ruff writes it: lowercase, `u` dropped, except `R`.
fn prefix_of(prefix: &str) -> String {
    let raw: String = prefix.chars().filter(|c| matches!(c, 'r' | 'R')).collect();
    let rest: String = prefix
        .chars()
        .filter(|c| !matches!(c, 'u' | 'U' | 'r' | 'R'))
        .map(|c| c.to_ascii_lowercase())
        .collect();
    format!("{raw}{rest}")
}

/// A string literal with normalized prefix and quotes (Black's `normalize_string_quotes`).
pub fn string<'a>(syn: &StringSyntax, text: &'a str, quote: Quote) -> Cow<'a, str> {
    let Some((prefix, q, body)) = split(syn, text) else {
        return Cow::Borrowed(text);
    };
    let new_prefix = prefix_of(prefix);
    let prefer = match quote {
        Quote::Single => '\'',
        _ => '"',
    };
    let q_char = q.chars().next().unwrap_or('"');
    let fstring = new_prefix.contains('f') || new_prefix.contains('t');
    let result = if quote == Quote::Preserve || (q.len() == 3 && q_char == prefer) {
        None
    } else {
        requote(syn, &new_prefix, q, q_char, prefer, body, fstring)
    };
    match result {
        Some(s) => Cow::Owned(s),
        None if new_prefix == prefix => Cow::Borrowed(text),
        None => Cow::Owned(format!("{new_prefix}{q}{body}{q}")),
    }
}

fn requote(
    syn: &StringSyntax,
    prefix: &str,
    q: &str,
    q_char: char,
    prefer: char,
    body: &str,
    fstring: bool,
) -> Option<String> {
    let esc = syn.escape.unwrap_or('\\');
    let triple = q.len() == 3;
    let (orig, new) = (q_char, if q_char == prefer { q_char } else { prefer });
    if orig == new {
        return Some(format!("{prefix}{q}{body}{q}"));
    }
    let new_q: String = std::iter::repeat_n(new, q.len()).collect();
    let raw = prefix.contains('r') || prefix.contains('R');
    let new_body: String = if raw {
        if body.contains(new) {
            return None;
        }
        body.to_string()
    } else {
        // (ruff keeps the escapes of a triple-quoted string: `\'''` stays)
        let body = if triple {
            Cow::Borrowed(body)
        } else {
            Cow::Owned(unescape_quote(body, orig, esc))
        };
        escape_quote(&body, new, triple, esc)
    };
    if fstring && has_backslash_in_field(&new_body, esc) {
        return None;
    }
    let mut new_body = new_body;
    if triple && new == '"' && new_body.ends_with(new) {
        new_body.pop();
        // (an odd run of backslashes already escapes it)
        let run = new_body.len() - new_body.trim_end_matches(esc).len();
        if run.is_multiple_of(2) {
            new_body.push(esc);
        }
        new_body.push(new);
    }
    let orig_esc = body.matches(esc).count();
    let new_esc = new_body.matches(esc).count();
    if new_esc > orig_esc || (new_esc == orig_esc && orig == prefer) {
        return None;
    }
    Some(format!("{prefix}{new_q}{new_body}{new_q}"))
}

/// `\'` (an escaped quote of the old kind) becomes `'`.
fn unescape_quote(body: &str, q: char, esc: char) -> String {
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars().peekable();
    let mut bs = 0;
    while let Some(c) = chars.next() {
        if c == esc {
            bs += 1;
            if chars.peek() == Some(&q) && bs % 2 == 1 {
                // An odd run's last backslash escapes the quote: drop only that one.
                bs = 0;
                continue;
            }
            out.push(c);
        } else {
            bs = 0;
            out.push(c);
        }
    }
    out
}

/// An unescaped `q` gets a backslash (in a triple-quoted string only a run of three would end it,
/// Black escapes every one).
fn escape_quote(body: &str, q: char, triple: bool, esc: char) -> String {
    let mut out = String::with_capacity(body.len() + 2);
    let mut bs = 0;
    let chars: Vec<char> = body.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if c == q && bs % 2 == 0 && (!triple || chars[i..].starts_with(&[q, q, q])) {
            out.push(esc);
        }
        bs = if c == esc { bs + 1 } else { 0 };
        out.push(c);
    }
    out
}

fn has_backslash_in_field(body: &str, esc: char) -> bool {
    let b = body.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'{' {
            if b.get(i + 1) == Some(&b'{') {
                i += 2;
                continue;
            }
            let mut depth = 1;
            let mut j = i + 1;
            while j < b.len() && depth > 0 {
                match b[j] {
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    c if c as char == esc => return true,
                    _ => {}
                }
                j += 1;
            }
            i = j;
        } else {
            i += 1;
        }
    }
    false
}

/// An f-string's or t-string's text as one string: Black's rule on its source, refused when a
/// replacement field holds a quote (a nested string).
pub fn fstring<'a>(syn: &StringSyntax, text: &'a str, quote: Quote) -> Cow<'a, str> {
    let Some((_, _, body)) = split(syn, text) else {
        return Cow::Borrowed(text);
    };
    if fields_hold_quotes(body) {
        let Some((prefix, q, body)) = split(syn, text) else {
            return Cow::Borrowed(text);
        };
        let p = prefix_of(prefix);
        let body = requote_fields(body, q, quote);
        return if p == prefix && body == split(syn, text).map_or("", |s| s.2) {
            Cow::Borrowed(text)
        } else {
            Cow::Owned(format!("{p}{q}{body}{q}"))
        };
    }
    string(syn, text, quote)
}

/// The body of an f-string with the quotes inside its replacement fields all spelled `"`, for
/// comparing values across the nested quote changes of [`fstring`].
pub fn field_quotes_value(body: &str) -> String {
    let mut depth = 0usize;
    body.chars()
        .map(|c| {
            match c {
                '{' => depth += 1,
                '}' => depth = depth.saturating_sub(1),
                _ => {}
            }
            if depth > 0 && c == '\'' { '"' } else { c }
        })
        .collect()
}

/// The strings in the replacement fields of a triple-quoted f-string that is in the preferred
/// quote: a string in the other quote that holds no quote itself takes the preferred one.
fn requote_fields<'a>(body: &'a str, outer: &str, quote: Quote) -> Cow<'a, str> {
    let (want, other) = match quote {
        Quote::Double => ('"', '\''),
        Quote::Single => ('\'', '"'),
        Quote::Preserve => return Cow::Borrowed(body),
    };
    if outer.len() != 3 || !outer.starts_with(want) || body.contains("'''") {
        return Cow::Borrowed(body);
    }
    let chars: Vec<char> = body.chars().collect();
    let mut out = String::with_capacity(body.len());
    let (mut i, mut depth) = (0, 0usize);
    while i < chars.len() {
        let c = chars[i];
        if depth == 0 && c == '{' && chars.get(i + 1) == Some(&'{') {
            out.push_str("{{");
            i += 2;
            continue;
        }
        match c {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            _ if depth > 0 && c == other => {
                let end = chars[i + 1..].iter().position(|x| *x == other);
                if let Some(n) = end.map(|n| i + 1 + n)
                    && !chars[i + 1..n]
                        .iter()
                        .any(|x| matches!(x, '\\' | '"' | '\'' | '{' | '}'))
                {
                    out.push(want);
                    out.extend(&chars[i + 1..n]);
                    out.push(want);
                    i = n + 1;
                    continue;
                }
            }
            _ => {}
        }
        out.push(c);
        i += 1;
    }
    if out == body {
        Cow::Borrowed(body)
    } else {
        Cow::Owned(out)
    }
}

fn fields_hold_quotes(body: &str) -> bool {
    let b = body.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'{' {
            if b.get(i + 1) == Some(&b'{') {
                i += 2;
                continue;
            }
            let mut depth = 1;
            let mut j = i + 1;
            while j < b.len() && depth > 0 {
                match b[j] {
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    b'"' | b'\'' => return true,
                    _ => {}
                }
                j += 1;
            }
            i = j;
        } else {
            i += 1;
        }
    }
    false
}

/// The comparison value of a string: prefix lowercased, outer quotes removed, escaped quotes
/// unescaped, so a quote change keeps it.
pub fn string_value(syn: &StringSyntax, text: &str) -> String {
    match split(syn, text) {
        Some((p, _, body)) => format!("{}{}", prefix_value(p), super::unescape_quotes(body, syn)),
        None => text.to_string(),
    }
}

/// A prefix without the spelling the formatter changes: lowercase, no `u`, letters in order.
pub fn prefix_value(prefix: &str) -> String {
    let mut chars: Vec<char> = prefix
        .to_ascii_lowercase()
        .chars()
        .filter(|c| *c != 'u')
        .collect();
    chars.sort_unstable();
    chars.into_iter().collect()
}
