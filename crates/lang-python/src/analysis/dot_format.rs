//! F521-F525: a `"...".format(...)` whose format string is invalid or does not fit the call.
//! The string is read as `str.format` does: fields `{name!conv:spec}` with at most one level of
//! fields in the spec, `{{` and `}}` escapes, `.attr` and `[index]` after the name.

use super::model::Model;
use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::Ranged;
use std::collections::BTreeSet;

#[derive(Default)]
struct Fields {
    /// Fields that take the next positional argument.
    auto: usize,
    /// Positional arguments named by number.
    index: BTreeSet<usize>,
    /// Keyword arguments named, once each, in order.
    keywords: Vec<String>,
}

enum Arg {
    Auto,
    Index(usize),
    Keyword(String),
}

/// The text of every field of `s` (nested ones in a spec included), or why `s` is invalid.
fn field_texts(s: &str) -> Result<Vec<&str>, &'static str> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < s.len() {
        let rest = &s[at..];
        let Some(i) = rest.find(['{', '}']) else {
            break;
        };
        let (c, after) = (rest.as_bytes()[i], &rest[i + 1..]);
        if after.as_bytes().first() == Some(&c) {
            at += i + 2;
            continue;
        }
        if c == b'}' {
            return Err("Single '}' encountered in format string");
        }
        // A field ends at the first `}` outside a nested `{...}`.
        let (mut nested, mut end) = (false, None);
        for (j, ch) in after.char_indices() {
            match ch {
                '{' if nested => return Err("Max format placeholder recursion exceeded"),
                '{' => nested = true,
                '}' if nested => nested = false,
                '}' => {
                    end = Some(j);
                    break;
                }
                _ => {}
            }
        }
        let Some(end) = end else {
            return Err("Single '{' encountered in format string");
        };
        out.push(&after[..end]);
        at += i + 1 + end + 1;
    }
    Ok(out)
}

/// The argument a field refers to; `text` is what is inside the braces, up to any `:`.
fn argument(name: &str) -> Result<Arg, &'static str> {
    let (head, mut rest) = name.split_at(name.find(['.', '[']).unwrap_or(name.len()));
    while !rest.is_empty() {
        if let Some(attr) = rest.strip_prefix('.') {
            let end = attr.find(['.', '[']).unwrap_or(attr.len());
            if end == 0 {
                return Err("Empty attribute in format string");
            }
            rest = &attr[end..];
        } else if let Some(index) = rest.strip_prefix('[') {
            let Some(end) = index.find(']') else {
                return Err("Expected '}' before end of string");
            };
            if end == 0 {
                return Err("Empty attribute in format string");
            }
            rest = &index[end + 1..];
            if !rest.is_empty() && !rest.starts_with(['.', '[']) {
                return Err("Only '.' or '[' may follow ']' in format field specifier");
            }
        }
    }
    Ok(if head.is_empty() {
        Arg::Auto
    } else if let Ok(i) = head.parse() {
        Arg::Index(i)
    } else {
        Arg::Keyword(head.to_string())
    })
}

fn fields(s: &str) -> Result<Fields, &'static str> {
    let mut out = Fields::default();
    for text in field_texts(s)? {
        let (name, spec) = text.split_once(':').unwrap_or((text, ""));
        let (name, conversion) = name
            .split_once('!')
            .map_or((name, None), |(n, c)| (n, Some(c)));
        if conversion.is_some_and(|c| c.chars().count() != 1) {
            return Err("Unexpected error parsing format string");
        }
        for name in std::iter::once(name).chain(
            field_texts(spec)?
                .into_iter()
                .map(|t| t.split([':', '!']).next().unwrap_or("")),
        ) {
            match argument(name)? {
                Arg::Auto => out.auto += 1,
                Arg::Index(i) => {
                    out.index.insert(i);
                }
                Arg::Keyword(k) => {
                    if !out.keywords.contains(&k) {
                        out.keywords.push(k);
                    }
                }
            }
        }
    }
    Ok(out)
}

fn join<T: ToString>(items: impl IntoIterator<Item = T>) -> String {
    items
        .into_iter()
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn check(m: &mut Model<'_>, c: &ast::ExprCall) {
    let Expr::Attribute(a) = &*c.func else { return };
    let Expr::StringLiteral(s) = &*a.value else {
        return;
    };
    if a.attr.as_str() != "format" {
        return;
    }
    let range = c.range();
    let f = match fields(s.value.to_str()) {
        Ok(f) => f,
        Err(why) => return m.hit("string-dot-format-invalid-format", range, why),
    };
    if f.auto > 0 && !f.index.is_empty() {
        m.hit("string-dot-format-mixing-automatic", range, "");
    }
    let args = &c.arguments.args;
    let used = |i: usize| i < f.auto || f.index.contains(&i);
    let unused: Vec<usize> = (0..args.len())
        .filter(|&i| !args[i].is_starred_expr() && !used(i))
        .collect();
    if !unused.is_empty() {
        m.hit(
            "string-dot-format-extra-positional-arguments",
            range,
            join(unused),
        );
    }
    let named: Vec<&str> = c
        .arguments
        .keywords
        .iter()
        .filter_map(|k| k.arg.as_ref().map(ast::Identifier::as_str))
        .collect();
    let splat = named.len() < c.arguments.keywords.len();
    if !splat {
        let extra: Vec<_> = named
            .iter()
            .filter(|n| !f.keywords.iter().any(|k| k == *n))
            .collect();
        if !extra.is_empty() {
            m.hit(
                "string-dot-format-extra-named-arguments",
                range,
                join(extra),
            );
        }
    }
    if !splat && !args.iter().any(Expr::is_starred_expr) {
        let positions = (0..f.auto)
            .chain(f.index.iter().copied())
            .filter(|&i| i >= args.len())
            .collect::<BTreeSet<_>>();
        let keywords = f.keywords.iter().filter(|k| !named.contains(&k.as_str()));
        let missing = join(
            positions
                .into_iter()
                .map(|i| i.to_string())
                .chain(keywords.cloned()),
        );
        if !missing.is_empty() {
            m.hit("string-dot-format-missing-arguments", range, missing);
        }
    }
}
