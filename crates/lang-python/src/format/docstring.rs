//! Docstrings: quotes, indentation and trailing whitespace as ruff writes them.
//!
//! concept: literal-spelling
//! what: Docstring quotes, indentation and trailing whitespace.
//! tools: ruff format docstrings, Black

use super::util::*;
use crate::generated::kind as k;
use core_format::FormatCtx;
use core_format::doc::concat;
use core_format::literals::{self, prefixed};
use core_syntax::SyntaxNode;

/// A docstring statement: the string with ruff's layout.
pub(crate) fn statement(cx: &FormatCtx<'_>, stmt: &SyntaxNode) -> R {
    let tokens: Vec<_> = stmt
        .descendants_with_tokens()
        .filter_map(|e| e.into_token())
        .filter(|t| t.kind() == k::STRING)
        .collect();
    let (Some(token), Some(last_token)) = (tokens.first(), tokens.last()) else {
        return Err(shape_error("docstring"));
    };
    let text = if tokens.len() == 1 {
        prefixed::string(syntax(), token.text(), style(cx).quote).into_owned()
    } else {
        // An implicit concatenation is the docstring when its parts join; else a plain string.
        let strings = stmt
            .children()
            .next()
            .ok_or_else(|| shape_error("docstring"))?;
        match super::expr::joined(cx, &strings) {
            Some(flat) => flat,
            None => return super::stmt::statement(cx, stmt),
        }
    };
    let Some((prefix, quote, body)) = prefixed::split(syntax(), &text) else {
        return Ok(cx.verbatim_node(stmt));
    };
    Ok(concat(vec![
        cx.leading_comments_doc(token),
        literals::docstring(prefix, quote, body),
        cx.trailing_comments_doc(last_token),
    ]))
}
