//! The family `unicode-escape`: escapes in a string or character literal (a token tagged
//! `#string`). The param `check` picks one:
//! - `visible`: a `\uXXXX` escape that writes a visible character, where the character itself
//!   could stand. An escape for a control, space, format or invisible character is fine, and so
//!   is a literal on a line that ends with a comment (the comment tells what the escape is);
//! - `short`: a `\uXXXX` escape, and with the flag `octal` a three-digit octal escape, of a
//!   character that has a short escape (`\n`, `\t`, `\"`); `short` lists those code points in
//!   decimal, separated by blanks.
//!
//! concept: unicode-escape
//! what: A unicode or octal escape in a string literal where the character is visible or has a short escape.
//! tools: Checkstyle AvoidEscapedUnicodeCharacters, IllegalTokenText
//! rules: avoid-unicode-escape, illegal-escape-text

use crate::{Family, FileContext, Params, Rule, RuleContext, RuleMeta};
use core_syntax::TextRange;

pub const FAMILY: Family = Family {
    name: "unicode-escape",
    build,
};

enum Check {
    Visible,
    Short { codes: Vec<u32>, octal: bool },
}

struct UnicodeEscape {
    meta: &'static RuleMeta,
    message: String,
    check: Check,
}

fn build(meta: &'static RuleMeta, mut p: Params) -> Result<Box<dyn Rule>, String> {
    let message = p.text("message")?;
    let visible = p.choice("check", true, &[("visible", true), ("short", false)])?;
    let check = if visible {
        Check::Visible
    } else {
        let codes = p
            .text("short")?
            .split_whitespace()
            .map(|c| {
                c.parse()
                    .map_err(|_| format!("`short`: `{c}` is not a number"))
            })
            .collect::<Result<_, _>>()?;
        Check::Short {
            codes,
            octal: p.flag("octal")?,
        }
    };
    p.done()?;
    Ok(Box::new(UnicodeEscape {
        meta,
        message,
        check,
    }))
}

/// Does the code point write nothing a reader could see (control, space, format, line breaks)?
fn invisible(c: u32) -> bool {
    matches!(c,
        0..=0x1f | 0x7f..=0xa0 | 0xad | 0x600..=0x605 | 0x61c | 0x6dd | 0x70f | 0x1680 | 0x180e
        | 0x2000..=0x200f | 0x2028..=0x202f | 0x205f..=0x2064 | 0x2066..=0x206f | 0x3000
        | 0xfeff | 0xfff9..=0xfffb)
}

/// The code point of the `\uXXXX` escape at the offset, with the escape's length.
fn unicode_at(text: &str, i: usize) -> Option<u32> {
    let b = text.as_bytes();
    if b.get(i) != Some(&b'\\') || b.get(i + 1) != Some(&b'u') || !text.is_char_boundary(i + 6) {
        return None;
    }
    u32::from_str_radix(text.get(i + 2..i + 6)?, 16).ok()
}

/// The code point of the octal escape of a leading digit up to 3 and two more at the offset.
fn octal_at(text: &str, i: usize) -> Option<u32> {
    let b = text.as_bytes();
    if b.get(i) != Some(&b'\\') || !text.is_char_boundary(i + 4) {
        return None;
    }
    let d = text.get(i + 1..i + 4)?;
    if !d.bytes().all(|c| (b'0'..=b'7').contains(&c)) || d.as_bytes()[0] > b'3' {
        return None;
    }
    u32::from_str_radix(d, 8).ok()
}

impl UnicodeEscape {
    /// The offset and length of the first escape of the literal the check reports.
    fn find(&self, text: &str) -> Option<(usize, usize)> {
        (0..text.len()).find_map(|i| match &self.check {
            Check::Visible => unicode_at(text, i)
                .filter(|c| !invisible(*c))
                .map(|_| (i, 6)),
            Check::Short { codes, octal } => unicode_at(text, i)
                .filter(|c| codes.contains(c))
                .map(|_| (i, 6))
                .or_else(|| {
                    octal_at(text, i)
                        .filter(|c| *octal && codes.contains(c))
                        .map(|_| (i, 4))
                }),
        })
    }
}

impl Rule for UnicodeEscape {
    fn meta(&self) -> &'static RuleMeta {
        self.meta
    }

    fn check_file(&self, file: &FileContext<'_>, ctx: &mut RuleContext<'_>) {
        if !file.text.contains('\\') {
            return;
        }
        let g = file.grammar;
        let visible = matches!(self.check, Check::Visible);
        for t in file
            .root
            .descendants_with_tokens()
            .filter_map(|e| e.into_token())
            .filter(|t| g.has_tag(t.kind(), "string") && t.text().contains('\\'))
        {
            let Some((at, len)) = self.find(t.text()) else {
                continue;
            };
            if visible {
                let end = usize::from(t.text_range().end());
                let rest = file.text[end..].split('\n').next().unwrap_or("");
                if rest.contains("//") || rest.contains("/*") {
                    continue;
                }
            }
            let start = usize::from(t.text_range().start()) + at;
            ctx.report(
                TextRange::new((start as u32).into(), ((start + len) as u32).into()),
                &self.message,
            );
        }
    }
}
