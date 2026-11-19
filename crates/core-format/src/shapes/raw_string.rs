//! A multi-line raw string that a call of the option `raw_string_calls` strips the indentation of
//! (`"""..."""` before `.trimIndent()`): its lines are written at the indentation of the code
//! around it, so the text keeps its meaning (the call removes what they have in common).
//!
//! concept: raw-string-reindent
//! what: Lines of a raw string followed by a trimIndent-like call at the indent of the code.
//! tools: ktlint string-template-indent
//! reads: tag:member-access

use crate::FormatCtx;
use crate::doc::{self, Doc};
use core_syntax::SyntaxNode;

/// The raw string `node` reindented, `None` when it is not one the option names a call for.
pub fn reindent(cx: &FormatCtx<'_>, node: &SyntaxNode) -> Option<Doc> {
    let g = cx.grammar;
    let call = |text: &str| cx.options.list_has("raw_string_calls", text);
    let parent = node
        .parent()
        .filter(|p| g.has_tag(p.kind(), "member-access"))?;
    let member = g.slot(&parent, "member")?;
    let name = member.as_node()?.first_token()?;
    if !call(name.text()) || parent.first_child().as_ref() != Some(node) {
        return None;
    }
    let text = node.text().to_string();
    let text = text.trim();
    if !text.starts_with("\"\"\"") || !text.contains('\n') {
        return None;
    }
    let lines: Vec<&str> = text.lines().collect();
    let indent = |l: &str| l.len() - l.trim_start().len();
    // (a last line of the closing quotes alone is no content: the call ignores its blanks)
    let last = lines.len() - 1;
    let bare_end = lines[last].trim() == "\"\"\"";
    let common = lines[1..if bare_end { last } else { lines.len() }]
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| indent(l))
        .min()
        .unwrap_or(0);
    let mut parts = vec![doc::text(lines[0].trim_end().to_string())];
    for (i, l) in lines.iter().enumerate().skip(1) {
        parts.push(doc::hardline());
        let cut = if bare_end && i == last {
            l.trim()
        } else {
            l.get(common.min(indent(l))..).unwrap_or("").trim_end()
        };
        parts.push(doc::text(cut.to_string()));
    }
    Some(doc::concat(parts))
}
