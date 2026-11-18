//! Quoted annotations (`x: "Foo"`, `List["Foo"]`): a string in a type position is code read
//! later, and the names in it are references from the scope the string is written in. The names
//! are the identifiers outside nested quotes and not behind a `.`; keywords are none.

use crate::scope_rules::ScopeLang;
use core_semantics::scopes::{Access, Model};
use core_syntax::roles::code_text;
use core_syntax::{Grammar, SyntaxNode, TextRange, TextSize};
use std::collections::HashSet;

/// Is the string a value, not a type: an item of a form such as `Literal[...]`, or a note of
/// `Annotated[T, ...]`?
fn is_value(g: &Grammar, lang: &ScopeLang, string: &SyntaxNode) -> bool {
    let tag = |n: &SyntaxNode, t: &str| g.has_tag(n.kind(), t);
    let ordered = lang.spec().ordered.as_ref().expect("ordered");
    let mut child = string.clone();
    for anc in string.ancestors().skip(1) {
        if tag(&anc, "transparent") {
            child = anc;
        } else if tag(&anc, "collection") {
            if anc.parent().is_some_and(|p| tag(&p, "index")) {
                child = anc;
                continue;
            }
            return false;
        } else if tag(&anc, "index") {
            let object = anc.first_child().map(|o| code_text(&o)).unwrap_or_default();
            let name = object.rsplit('.').next().unwrap_or("");
            if ordered.value_forms.contains(&name) {
                return true;
            }
            return ordered.note_forms.contains(&name)
                && match anc.children().nth(1) {
                    Some(s) if tag(&s, "collection") => {
                        s.children().next().is_none_or(|first| first != child)
                    }
                    _ => false,
                };
        } else {
            return false;
        }
    }
    false
}

/// Drops the strings that are values from the model's texts and adds a read for each name in
/// the others.
pub(in crate::scope_rules) fn add_reads(
    model: &mut Model,
    root: &SyntaxNode,
    g: &Grammar,
    lang: &ScopeLang,
) {
    model.texts.retain(|t| !is_value(g, lang, &t.node(root)));
    let keywords: HashSet<&str> = g
        .kinds_with_tag("keyword")
        .filter_map(|d| d.text.as_deref())
        .collect();
    let texts: Vec<_> = model
        .texts
        .iter()
        .map(|t| (t.node(root), t.scope))
        .collect();
    for (string, scope) in texts {
        let mut parts = string.children();
        let (Some(part), None) = (parts.next(), parts.next()) else {
            continue;
        };
        let Some(token) = part
            .descendants_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| g.has_tag(t.kind(), "string"))
        else {
            continue;
        };
        let text = token.text();
        let Some(quote) = text.find(['\'', '"']) else {
            continue;
        };
        let prefix = &text[..quote];
        let width = if text.len() - quote >= 6
            && text[quote..].starts_with(&text[quote..quote + 1].repeat(3))
        {
            3
        } else {
            1
        };
        let Some(inner) = text.get(quote + width..text.len().saturating_sub(width)) else {
            continue;
        };
        if prefix.contains(['b', 'B']) || inner.contains('\\') {
            continue;
        }
        let start = token.text_range().start() + TextSize::new((quote + width) as u32);
        for (i, end) in names(inner, &keywords).unwrap_or_default() {
            let at = start + TextSize::new(i as u32);
            model.add_reference(
                &inner[i..end],
                TextRange::at(at, TextSize::new((end - i) as u32)),
                (string.text_range(), string.kind()),
                Access::READ.union(Access::TYPE),
                scope,
            );
        }
    }
}

/// The names written in `text` as spans: the identifiers outside nested quotes and not behind a
/// `.`, that are no keyword. `None` when the text cannot be one expression (brackets that do
/// not match, two operands in a row).
fn names(text: &str, keywords: &HashSet<&str>) -> Option<Vec<(usize, usize)>> {
    let (mut found, mut depth, mut quoted) = (Vec::new(), 0i32, false);
    // The last thing written ends an operand; whitespace came after it.
    let (mut operand, mut spaced) = (false, false);
    let mut chars = text.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if c == '\'' || c == '"' {
            if !quoted && operand && spaced {
                return None;
            }
            quoted = !quoted;
            (operand, spaced) = (!quoted, false);
        } else if quoted {
        } else if c.is_whitespace() {
            spaced = operand;
        } else if c.is_alphanumeric() || c == '_' {
            let mut end = i + c.len_utf8();
            while let Some(&(j, d)) = chars.peek() {
                if !(d.is_alphanumeric() || d == '_') {
                    break;
                }
                end = j + d.len_utf8();
                chars.next();
            }
            let word = &text[i..end];
            if keywords.contains(word) {
                (operand, spaced) = (false, false);
                continue;
            }
            if operand && spaced {
                return None;
            }
            let member = text[..i].trim_end().ends_with('.');
            if !member && (c.is_alphabetic() || c == '_') {
                found.push((i, end));
            }
            (operand, spaced) = (true, false);
        } else {
            match c {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                _ => {}
            }
            if depth < 0 {
                return None;
            }
            (operand, spaced) = (matches!(c, ')' | ']' | '}'), false);
        }
    }
    (depth == 0 && !quoted).then_some(found)
}
