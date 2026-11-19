//! The text of embedded code in the host's lines: the code without the host's blank lines and
//! shared indentation, and its lines as `Doc`s (the host's indentation is the printer's). The
//! lines that start inside a template literal keep their text: their indentation is part of the
//! string; `quote` is the character that opens and closes one (a fact of the embedded language).
//!
//! concept: embedded-code
//! what: Code of another language inside a file, reported by the host provider and parsed by that language.

use crate::context::verbatim_text;
use crate::doc::{self, Doc};

/// `code` without the blank lines before it and the blanks after it (the first line's own
/// indentation stays); `blank` says which characters are blank.
pub fn trim_blank_lines(code: &str, blank: fn(char) -> bool) -> &str {
    let mut v = code.trim_end_matches(blank);
    while let Some(i) = v.find('\n') {
        if v[..i].chars().all(blank) {
            v = &v[i + 1..];
        } else {
            break;
        }
    }
    v
}

/// Per line of `code`: does it start inside a template literal?
pub fn starts_in_template(code: &str, quote: char) -> Vec<bool> {
    let (mut inside, mut escaped) = (false, false);
    code.split('\n')
        .map(|line| {
            let start = inside;
            for c in line.chars() {
                match c {
                    '\\' => escaped = !escaped,
                    c if c == quote && !escaped => inside = !inside,
                    _ => escaped = false,
                }
            }
            start
        })
        .collect()
}

/// `code` without the indentation all its lines share; the lines inside a template literal stay.
pub fn dedent(code: &str, quote: char) -> String {
    let indent = |l: &str| l.chars().take_while(|c| matches!(c, ' ' | '\t')).count();
    let inside = starts_in_template(code, quote);
    let kept = || code.split('\n').zip(&inside).filter(|(_, i)| !**i);
    let min = kept()
        .filter(|(l, _)| !l.trim().is_empty())
        .map(|(l, _)| indent(l))
        .min()
        .unwrap_or(0);
    code.split('\n')
        .zip(&inside)
        .map(|(l, i)| {
            if *i {
                l.to_string()
            } else {
                l.chars().skip(min.min(indent(l))).collect::<String>()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The lines of `code`, one per line, except the lines that start inside a template literal: they
/// join the line before.
pub fn lines(code: &str, quote: char) -> Doc {
    let mut chunks = Vec::<String>::new();
    for (line, inside) in code.split('\n').zip(starts_in_template(code, quote)) {
        match chunks.last_mut() {
            Some(last) if inside => {
                last.push('\n');
                last.push_str(line);
            }
            _ => chunks.push(line.to_string()),
        }
    }
    doc::join(&doc::hardline(), chunks.iter().map(|c| verbatim_text(c)))
}
