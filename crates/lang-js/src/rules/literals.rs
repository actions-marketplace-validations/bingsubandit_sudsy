//! Rules about the text of literals: irregular whitespace and number precision.
//!
//! concept: new no-irregular-whitespace, new no-loss-of-precision
//! what: no-irregular-whitespace: Irregular whitespace characters in source.
//! what: no-loss-of-precision: Number literals that lose precision.

use super::{Check, code_text, span};
use crate::generated::kind as k;
use core_rules::RuleContext;
use core_syntax::{SyntaxNode, TextRange, TextSize};

fn at(start: TextSize, offset: usize, len: usize) -> TextRange {
    TextRange::at(
        start + TextSize::from(offset as u32),
        TextSize::from(len as u32),
    )
}

fn irregular(ch: char) -> bool {
    matches!(
        ch,
        '\u{b}' | '\u{c}' | '\u{85}' | '\u{a0}' | '\u{1680}' | '\u{180e}' | '\u{2000}'
            ..='\u{200b}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

/// Whitespace other than space, tab and line breaks, outside strings (a BOM at the start is not
/// part of the text ESLint reads).
pub(super) fn irregular_whitespace(c: &Check, root: &SyntaxNode, ctx: &mut RuleContext<'_>) {
    let text = ctx.file.text;
    if text.bytes().all(|b| b.is_ascii() && b != 0x0b && b != 0x0c) {
        return;
    }
    let strings: Vec<TextRange> = (root.descendants_with_tokens())
        .filter_map(|e| e.into_token())
        .filter(|t| t.kind() == k::STRING)
        .map(|t| t.text_range())
        .collect();
    let mut run: Option<usize> = None;
    for (i, ch) in text
        .char_indices()
        .chain(std::iter::once((text.len(), 'a')))
    {
        match (run, irregular(ch) && !(i == 0 && ch == '\u{feff}')) {
            (None, true) => run = Some(i),
            (Some(start), false) => {
                run = None;
                let range = at(TextSize::from(0), start, i - start);
                if !strings.iter().any(|s| s.contains(range.start())) {
                    c.hit(ctx, range, "");
                }
            }
            _ => {}
        }
    }
}

/// Does the number literal `raw` not read back as itself?
fn loses_precision(raw: &str) -> bool {
    let s = raw.replace('_', "").to_ascii_lowercase();
    let legacy_octal =
        s.len() > 1 && s.starts_with('0') && s.bytes().all(|b| (b'0'..=b'7').contains(&b));
    let (radix, digits) = match (s.get(..2), s.as_str()) {
        (Some("0x"), _) => (16, &s[2..]),
        (Some("0o"), _) => (8, &s[2..]),
        (Some("0b"), _) => (2, &s[2..]),
        (_, _) if legacy_octal => (8, &s[1..]),
        _ => (10, s.as_str()),
    };
    if radix != 10 {
        // the value is exact when its significant bits (up to the last 1) fit in 53
        let bits: String = digits
            .chars()
            .map(|d| {
                format!(
                    "{:0w$b}",
                    d.to_digit(radix).unwrap_or(0),
                    w = radix.ilog2() as usize
                )
            })
            .collect();
        let bits = bits.trim_start_matches('0');
        return bits.len() > 1024 || bits.trim_end_matches('0').len() > 53;
    }
    let (mantissa, _) = s.split_once('e').unwrap_or((&s, ""));
    let significant = mantissa.replace('.', "");
    let significant = significant.trim_start_matches('0').trim_end_matches('0');
    if significant.is_empty() {
        return false;
    }
    let Ok(value) = s.parse::<f64>() else {
        return false;
    };
    if !value.is_finite() {
        return true;
    }
    let formatted = format!("{:.*e}", significant.len() - 1, value);
    let digits = formatted
        .split_once('e')
        .map_or("", |m| m.0)
        .replace('.', "");
    digits.trim_end_matches('0') != significant
}

pub(super) fn loss_of_precision(c: &Check, n: &SyntaxNode, ctx: &mut RuleContext<'_>) {
    if loses_precision(&code_text(n)) {
        c.hit(ctx, span(ctx, n), "");
    }
}
