//! The value of a string token: its quotes gone and its escapes decoded.

use std::borrow::Cow;

/// The text of a quoted string token (quotes included) with its escapes decoded: `'literal'` as
/// written, `"basic"` with `\n \t \r \b \f \uXXXX \UXXXXXXXX` and any other escaped character as
/// itself. A lone surrogate or a malformed hex escape becomes U+FFFD.
pub fn decode(token: &str) -> Cow<'_, str> {
    let inner = token.get(1..token.len().saturating_sub(1)).unwrap_or(token);
    if token.starts_with('\'') || !inner.contains('\\') {
        return Cow::Borrowed(inner);
    }
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('u') => out.push(unit(&mut chars)),
            Some('U') => out.push(
                hex(&mut chars, 8)
                    .and_then(char::from_u32)
                    .unwrap_or('\u{FFFD}'),
            ),
            Some(e) => out.push(match e {
                'b' => '\u{8}',
                't' => '\t',
                'n' => '\n',
                'f' => '\u{c}',
                'r' => '\r',
                other => other,
            }),
            None => out.push('\\'),
        }
    }
    Cow::Owned(out)
}

fn hex(chars: &mut std::str::Chars<'_>, digits: usize) -> Option<u32> {
    (0..digits).try_fold(0u32, |v, _| Some(v * 16 + chars.next()?.to_digit(16)?))
}

/// The character of a `\u` escape, joining a surrogate pair with the `\u` escape after it.
fn unit(chars: &mut std::str::Chars<'_>) -> char {
    let decoded = match hex(chars, 4) {
        Some(high @ 0xD800..=0xDBFF) => {
            let mut ahead = chars.clone();
            let low = (ahead.next() == Some('\\') && ahead.next() == Some('u'))
                .then(|| hex(&mut ahead, 4))
                .flatten()
                .filter(|l| (0xDC00..=0xDFFF).contains(l));
            low.and_then(|low| {
                *chars = ahead;
                char::from_u32(0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00))
            })
        }
        Some(u) => char::from_u32(u),
        None => None,
    };
    decoded.unwrap_or('\u{FFFD}')
}

/// A number's text without its `_` separators (borrowed when it has none).
pub fn digits(raw: &str) -> Cow<'_, str> {
    if raw.contains('_') {
        Cow::Owned(raw.replace('_', ""))
    } else {
        Cow::Borrowed(raw)
    }
}

/// The value of an integer token (`1_000`, `0xff`, `0o17`, `0b11`); `None` for anything else.
pub fn integer(raw: &str) -> Option<i128> {
    let digits = digits(raw);
    let radix = |p: &str, base| i128::from_str_radix(digits.strip_prefix(p)?, base).ok();
    radix("0x", 16)
        .or_else(|| radix("0o", 8))
        .or_else(|| radix("0b", 2))
        .or_else(|| digits.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::decode;

    #[test]
    fn escapes_decode() {
        assert_eq!(decode(r#""a""#), "a");
        assert_eq!(decode(r#""a\/\n""#), "a/\n");
        assert_eq!(decode(r#""😀""#), "\u{1F600}");
        assert_eq!(decode(r#""\ud83d""#), "\u{FFFD}");
        assert_eq!(decode(r#""😀""#), "\u{1F600}");
        assert_eq!(decode(r"'a\n'"), r"a\n");
    }
}
