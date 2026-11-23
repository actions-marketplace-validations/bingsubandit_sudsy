//! Is an attribute value one the language's metadata allows? The allowed values are items
//! separated by spaces: `a|b|c` (listed values, any case) or `/source/` and `/source/i` (a regex
//! source of the tool's dialect, `i`: any case). The sources `^\S+$`, `^.+$`, `^\d+$` and `^.*$`
//! are answered by a plain test; the others are built once per process, on first use. A list
//! attribute is checked token by token; an attribute that may be written empty is not wrong when
//! empty; a value that is a template expression (`{id}`, `id={expr}`) is not text and is not
//! judged.
//!
//! concept: allowed-values
//! what: An attribute value that the metadata's list or pattern does not allow.
//! tools: html-validate attribute-allowed-values

use crate::Re;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

/// Does one pattern item `/source/` or `/source/i` match `value`?
fn pattern_matches(item: &'static str, value: &str) -> bool {
    let (source, any_case) = match item.strip_suffix('i') {
        Some(rest) if rest.ends_with('/') => (rest, true),
        _ => (item, false),
    };
    match source.trim_matches('/') {
        "^\\S+$" => !value.is_empty() && !value.contains(char::is_whitespace),
        "^.+$" => !value.is_empty() && !value.contains('\n'),
        "^\\d+$" => !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()),
        "^.*$" => !value.contains('\n'),
        text => {
            static CACHE: LazyLock<Mutex<HashMap<&'static str, Option<Re>>>> =
                LazyLock::new(Default::default);
            let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
            cache
                .entry(item)
                .or_insert_with(|| {
                    let text = text.replace("[^]", "[\\s\\S]");
                    Re::lazy(&if any_case {
                        format!("(?i:{text})")
                    } else {
                        text
                    })
                    .ok()
                })
                .as_ref()
                .is_none_or(|re| re.is_match(value))
        }
    }
}

/// Does `value` match one of the items of `allowed`?
pub fn matches(allowed: &'static str, value: &str) -> bool {
    allowed.split(' ').any(|item| {
        if item.starts_with('/') {
            pattern_matches(item, value)
        } else {
            item.split('|').any(|v| v.eq_ignore_ascii_case(value))
        }
    })
}

/// Is `value` (`None`: written without `=`) wrong for `allowed`? `around` is the text before the
/// attribute name and after it; `omit` says whether an empty value is fine, `list` whether the
/// attribute is a token list (both asked only when needed).
pub fn wrong(
    allowed: &'static str,
    value: Option<&str>,
    around: (&str, &str),
    omit: impl Fn() -> bool,
    list: impl Fn() -> bool,
) -> bool {
    let text = value.unwrap_or("");
    if !allowed.split(' ').any(|item| item.starts_with('/')) {
        return value.is_some() && !text.is_empty() && !matches(allowed, text);
    }
    if value.is_some() && matches(allowed, text) {
        return false;
    }
    let expression =
        around.0.ends_with('{') || (value.is_none() && around.1.trim_start().starts_with('='));
    if expression || (text.is_empty() && omit()) {
        return false;
    }
    !(value.is_some() && list() && text.split_whitespace().all(|v| matches(allowed, v)))
}
