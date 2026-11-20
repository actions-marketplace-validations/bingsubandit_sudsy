//! A docstring's text as ruff and Black write it: the lines re-indented to the same margin,
//! trailing blanks and blank lines after a single line of text gone, a space where the closing
//! quotes would merge with the text.
//!
//! concept: docstring-reindent
//! what: Docstring quotes, indentation and trailing whitespace.
//! tools: ruff format docstrings, Black

use crate::doc::{self, Doc, concat, hardline};

/// The docstring `prefix` `quote` `body` `quote`, laid out at the indentation of its statement.
pub fn docstring(prefix: &str, quote: &str, body: &str) -> Doc {
    let empty = body.is_empty();
    let body = clean(body);
    // Blank lines after a single line of text go: `"""text\n"""` is `"""text"""`.
    let flat = body.trim_end();
    let lines: Vec<&str> = if flat.contains('\n') { &body } else { flat }
        .split('\n')
        .collect();
    let mut parts: Vec<Doc> = vec![doc::text(format!("{prefix}{quote}"))];
    let last = lines.len() - 1;
    if lines.len() == 1 {
        let l = lines[0].trim();
        if l.is_empty() {
            if !empty {
                parts.push(doc::text(" "));
            }
        } else {
            if l.starts_with(quote.chars().next().unwrap_or('"')) {
                parts.push(doc::text(" "));
            }
            parts.push(doc::text(l.to_string()));
            if needs_space(l, quote) {
                parts.push(doc::text(" "));
            }
        }
        parts.push(doc::text(quote.to_string()));
        return concat(parts);
    }
    let indent_of = |l: &str| l.len() - l.trim_start().len();
    let min = lines[1..]
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| indent_of(l))
        .min()
        .unwrap_or(0);
    parts.push(doc::text(lines[0].trim().to_string()));
    let closing_alone = lines[last].trim().is_empty();
    let end = if closing_alone { last } else { last + 1 };
    // Trailing blank lines before the closing quotes go.
    let stop = end;
    for l in &lines[1..stop] {
        parts.push(hardline());
        if l.trim().is_empty() {
            continue;
        }
        parts.push(doc::text(l[min.min(indent_of(l))..].trim_end().to_string()));
    }
    if closing_alone {
        parts.push(hardline());
    } else if let Some(l) = lines[1..stop].last()
        && needs_space(l.trim_end(), quote)
    {
        parts.push(doc::text(" "));
    }
    parts.push(doc::text(quote.to_string()));
    concat(parts)
}

/// A space keeps the closing quotes from merging with the text: it ends in an unescaped quote
/// character or in a backslash.
fn needs_space(l: &str, quote: &str) -> bool {
    let q = quote.chars().next().unwrap_or('"');
    l.strip_suffix(q)
        .is_some_and(|rest| !ends_with_odd_backslash(rest))
        || ends_with_odd_backslash(l)
}

fn ends_with_odd_backslash(l: &str) -> bool {
    l.chars().rev().take_while(|c| *c == '\\').count() % 2 == 1
}

/// The docstring with the leading tabs of its lines expanded to spaces.
fn clean(body: &str) -> String {
    body.replace("\r\n", "\n")
        .split('\n')
        .enumerate()
        .map(|(i, l)| {
            if i == 0 {
                return l.to_string();
            }
            let ws = l.len() - l.trim_start_matches([' ', '\t']).len();
            let mut col = 0;
            for c in l[..ws].chars() {
                if c == '\t' {
                    col = (col / 8 + 1) * 8;
                } else {
                    col += 1;
                }
            }
            format!("{}{}", " ".repeat(col), &l[ws..])
        })
        .collect::<Vec<_>>()
        .join("\n")
}
