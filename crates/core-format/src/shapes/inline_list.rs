//! Items separated by commas with no delimiters of their own (a tuple without parentheses, an
//! unparenthesized list of targets or values): one line when they fit, else one item per line
//! with a comma after the last. The delimiters, when the caller adds them, wrap the result.
//!
//! concept: delimited-list
//! what: Comma-separated items without delimiters, flat or one per line with a trailing comma.
//! tools: ruff tuples without parentheses

use crate::doc::{self, Doc};

/// One item and the separator the source has after it.
pub struct InlineItem {
    pub doc: Doc,
    /// The separator printed after the item (when the source has one).
    pub sep: Option<Doc>,
    /// A comment stands on that separator: it stays after the last item.
    pub sep_commented: bool,
}

pub struct InlineStyle {
    /// The items go on lines of their own when they do not fit; otherwise they stay on one line
    /// and only the brackets inside them split.
    pub breakable: bool,
    /// The items form a group of their own; else they break with the group around them.
    pub grouped: bool,
    /// The source has a separator after the last item: the list stays expanded.
    pub magic: bool,
    /// The separator after the last item when the list breaks, and for a single item always.
    pub separator: &'static str,
}

pub fn inline_list(items: Vec<InlineItem>, style: &InlineStyle) -> Doc {
    let n = items.len();
    let mut parts = Vec::new();
    for (i, item) in items.into_iter().enumerate() {
        if i > 0 {
            parts.push(if style.breakable {
                doc::line()
            } else {
                doc::lit(" ")
            });
        }
        parts.push(item.doc);
        let last = i + 1 == n;
        match item.sep {
            Some(sep) if !last || item.sep_commented => parts.push(sep),
            _ if last && n == 1 => parts.push(doc::lit(style.separator)),
            _ if last && style.breakable => {
                parts.push(doc::if_break(doc::lit(style.separator), Doc::Nil))
            }
            _ => {}
        }
    }
    let body = doc::concat(parts);
    if style.breakable && style.grouped {
        if style.magic && n > 1 {
            doc::group_broken(body)
        } else {
            doc::group(body)
        }
    } else {
        body
    }
}
