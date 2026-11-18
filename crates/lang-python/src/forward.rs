//! Quoted annotations (`x: "Foo"`, `List["Foo"]`, `cast("Foo", x)`) that are not one Python
//! expression: the string is code, parsed with the Python grammar (the shared scope model reads
//! its names, `core_rules::scope_rules`).
//!
//! concept: embedded-code

use crate::adapter::parse_snippet;
use crate::generated::kind as k;
use crate::semantic::Cx;
use crate::text::{code_text, parts};
use core_syntax::TextRange;

/// Where the quoted annotations of the file are no expression, and why.
pub fn syntax_errors(cx: &Cx) -> Vec<(TextRange, String)> {
    let mut errors = Vec::new();
    for t in &cx.s.texts {
        let string = t.node(&cx.root);
        let range = cx.range(&string);
        let pieces = parts(&string);
        let [piece] = pieces.as_slice() else {
            if pieces.len() > 1 {
                errors.push((range, code_text(&string)));
            }
            continue;
        };
        if piece.bytes || piece.inner.contains('\\') {
            continue;
        }
        let parsed = parse_snippet(&piece.inner);
        let body: Vec<_> = parsed.tree.children().collect();
        let fails = !matches!(body.as_slice(), [stmt] if stmt.kind() == k::STMT_EXPR)
            || !parsed.errors.is_empty();
        if fails {
            errors.push((range, piece.inner.clone()));
        }
    }
    errors
}
