//! Strings and numbers: quote and prefix normalization, number spelling, docstrings.

use std::borrow::Cow;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Quote {
    Double,
    Single,
    Preserve,
}

/// A string token's pieces: prefix, quote (1 or 3 characters) and body.
pub(crate) fn split(text: &str) -> Option<(&str, &str, &str)> {
    let start = text.find(['"', '\''])?;
    let (prefix, rest) = text.split_at(start);
    let q = if rest.len() >= 6 && (rest.starts_with("\"\"\"") || rest.starts_with("'''")) {
        3
    } else {
        1
    };
    if rest.len() < 2 * q {
        return None;
    }
    Some((prefix, &rest[..q], &rest[q..rest.len() - q]))
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
pub(crate) fn string(text: &str, quote: Quote) -> Cow<'_, str> {
    let Some((prefix, q, body)) = split(text) else {
        return Cow::Borrowed(text);
    };
    let new_prefix = prefix_of(prefix);
    let (prefer, other) = match quote {
        Quote::Single => ('\'', '"'),
        _ => ('"', '\''),
    };
    let q_char = q.chars().next().unwrap_or('"');
    let fstring = new_prefix.contains('f') || new_prefix.contains('t');
    let result = if quote == Quote::Preserve || (q.len() == 3 && q_char == prefer) {
        None
    } else {
        requote(&new_prefix, q, q_char, prefer, other, body, fstring)
    };
    match result {
        Some(s) => Cow::Owned(s),
        None if new_prefix == prefix => Cow::Borrowed(text),
        None => Cow::Owned(format!("{new_prefix}{q}{body}{q}")),
    }
}

fn requote(
    prefix: &str,
    q: &str,
    q_char: char,
    prefer: char,
    _other: char,
    body: &str,
    fstring: bool,
) -> Option<String> {
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
            Cow::Owned(unescape_quote(body, orig))
        };
        escape_quote(&body, new, triple)
    };
    if fstring && has_backslash_in_field(&new_body) {
        return None;
    }
    let mut new_body = new_body;
    if triple && new == '"' && new_body.ends_with(new) {
        new_body.pop();
        // (an odd run of backslashes already escapes it)
        let run = new_body.len() - new_body.trim_end_matches('\\').len();
        new_body.push_str(if run % 2 == 1 { "\"" } else { "\\\"" });
    }
    let orig_esc = body.matches('\\').count();
    let new_esc = new_body.matches('\\').count();
    if new_esc > orig_esc || (new_esc == orig_esc && orig == prefer) {
        return None;
    }
    Some(format!("{prefix}{new_q}{new_body}{new_q}"))
}

/// `\'` (an escaped quote of the old kind) becomes `'`.
fn unescape_quote(body: &str, q: char) -> String {
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars().peekable();
    let mut bs = 0;
    while let Some(c) = chars.next() {
        if c == '\\' {
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
fn escape_quote(body: &str, q: char, triple: bool) -> String {
    let mut out = String::with_capacity(body.len() + 2);
    let mut bs = 0;
    let chars: Vec<char> = body.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if c == q && bs % 2 == 0 && (!triple || chars[i..].starts_with(&[q, q, q])) {
            out.push('\\');
        }
        bs = if c == '\\' { bs + 1 } else { 0 };
        out.push(c);
    }
    out
}

fn has_backslash_in_field(body: &str) -> bool {
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
                    b'\\' => return true,
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
pub(crate) fn fstring(text: &str, quote: Quote) -> Cow<'_, str> {
    let Some((_, _, body)) = split(text) else {
        return Cow::Borrowed(text);
    };
    if fields_hold_quotes(body) {
        let Some((prefix, q, body)) = split(text) else {
            return Cow::Borrowed(text);
        };
        let p = prefix_of(prefix);
        return if p == prefix {
            Cow::Borrowed(text)
        } else {
            Cow::Owned(format!("{p}{q}{body}{q}"))
        };
    }
    string(text, quote)
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
pub(crate) fn string_value(text: &str) -> String {
    match split(text) {
        Some((p, _, body)) => format!("{}{}", prefix_value(p), unescape_quotes(body)),
        None => text.to_string(),
    }
}

/// The body with `\"` and `\'` as plain quotes (a backslash pair stays).
fn unescape_quotes(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some(q @ ('"' | '\'')) => out.push(q),
                Some(o) => {
                    out.push(c);
                    out.push(o);
                }
                None => out.push(c),
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// A number as ruff writes it: lowercase prefix and exponent, uppercase hex digits, `j` suffix
/// lowercase, no trailing `.0` change.
pub(crate) fn number(text: &str) -> Cow<'_, str> {
    let lower = text.to_ascii_lowercase();
    let out = if let Some(rest) = lower
        .strip_prefix("0x")
        .map(|r| ("0x", r))
        .or_else(|| lower.strip_prefix("0o").map(|r| ("0o", r)))
        .or_else(|| lower.strip_prefix("0b").map(|r| ("0b", r)))
    {
        if rest.0 == "0x" {
            format!("0x{}", rest.1.to_ascii_uppercase())
        } else {
            lower.clone()
        }
    } else {
        // Decimal: lowercase `e` and `j`; a leading or trailing dot gets a zero.
        let (mant, tail) = match lower.find(['e', 'j']) {
            Some(i) => lower.split_at(i),
            None => (lower.as_str(), ""),
        };
        let mut m = mant.to_string();
        if m.starts_with('.') {
            m.insert(0, '0');
        }
        if m.ends_with('.') {
            m.push('0');
        }
        let tail = tail.replace("e+", "e");
        format!("{m}{tail}")
    };
    if out == text {
        Cow::Borrowed(text)
    } else {
        Cow::Owned(out)
    }
}

/// A prefix without the spelling the formatter changes: lowercase, no `u`, letters in order.
fn prefix_value(prefix: &str) -> String {
    let mut chars: Vec<char> = prefix
        .to_ascii_lowercase()
        .chars()
        .filter(|c| *c != 'u')
        .collect();
    chars.sort_unstable();
    chars.into_iter().collect()
}

/// What the safety check compares of a restyled token (see `PythonFormatter::value`).
pub(crate) fn value(t: &core_syntax::SyntaxToken) -> Cow<'_, str> {
    use crate::generated::kind as k;
    match t.kind() {
        k::INT | k::FLOAT | k::COMPLEX => Cow::Owned(number(t.text()).replace('_', "")),
        k::STRING => {
            let v = string_value(t.text());
            let statement = t
                .parent_ancestors()
                .nth(2)
                .is_some_and(|p| p.kind() == k::STMT_EXPR);
            if statement {
                Cow::Owned(
                    v.split('\n')
                        .map(str::trim)
                        .collect::<Vec<_>>()
                        .join("\n")
                        .trim()
                        .to_string(),
                )
            } else {
                Cow::Owned(v)
            }
        }
        k::F_STRING_START | k::T_STRING_START => {
            Cow::Owned(prefix_value(t.text().trim_end_matches(['"', '\''])))
        }
        k::F_STRING_END | k::T_STRING_END => Cow::Borrowed(""),
        k::F_STRING_MIDDLE | k::T_STRING_MIDDLE => Cow::Owned(unescape_quotes(t.text())),
        _ => Cow::Borrowed(t.text()),
    }
}

/// The value of a string expression: the prefix of its first part and the bodies of all parts,
/// so that joining an implicit concatenation keeps it (the safety check compares this).
pub(crate) fn node_value(n: &core_syntax::SyntaxNode) -> String {
    use crate::generated::kind as k;
    let mut out = String::new();
    for (i, part) in n.children().enumerate() {
        let text: String = part
            .descendants_with_tokens()
            .filter_map(|e| e.into_token())
            .filter(|t| {
                !matches!(
                    t.kind(),
                    k::WHITESPACE | k::NEWLINE | k::COMMENT | k::CONTINUATION
                )
            })
            .map(|t| t.text().to_string())
            .collect();
        match split(&text) {
            Some((p, _, body)) => {
                if i == 0 {
                    out.push_str(&prefix_value(p));
                }
                out.push_str(&unescape_quotes(body));
            }
            None => out.push_str(&text),
        }
    }
    // A docstring is compared without the indentation and trailing blanks the formatter changes.
    if n.parent().is_some_and(|p| p.kind() == k::STMT_EXPR) {
        return out
            .split('\n')
            .map(str::trim)
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string();
    }
    out
}
