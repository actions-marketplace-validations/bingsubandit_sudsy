//! A multi-line raw string that a call of the option `raw_string_calls` strips the indentation of
//! (`"""..."""` before `.trimIndent()`): its lines are written at the indentation of the code
//! around it, so the text keeps its meaning (the call removes what they have in common). Any other
//! raw string keeps its bytes, and the safety check compares the lines of only these without
//! their indentation.
//!
//! concept: raw-string-reindent
//! what: Lines of a raw string followed by a trimIndent-like call at the indent of the code.
//! tools: ktlint string-template-indent
//! reads: tag:member-access

use crate::FormatCtx;
use crate::doc::{self, Doc};
use core_config::ResolvedOptions;
use core_syntax::{Grammar, SyntaxNode};

/// The text of `node` when it is a multi-line raw string that is the object of a call named by
/// the option `raw_string_calls`.
fn target(options: &ResolvedOptions, g: &Grammar, node: &SyntaxNode) -> Option<String> {
    let parent = node
        .parent()
        .filter(|p| g.has_tag(p.kind(), "member-access"))?;
    let member = g.slot(&parent, "member")?;
    let name = member.as_node()?.first_token()?;
    if !options.list_has(core_config::opt::RAW_STRING_CALLS, name.text())
        || parent.first_child().as_ref() != Some(node)
    {
        return None;
    }
    let text = node.text().to_string();
    let text = text.trim();
    (text.starts_with("\"\"\"") && text.contains('\n')).then(|| text.to_string())
}

/// The raw string `node` reindented, `None` when it is not one the option names a call for.
pub fn reindent(cx: &FormatCtx<'_>, node: &SyntaxNode) -> Option<Doc> {
    let text = target(cx.options, cx.grammar, node)?;
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
    let mut parts = vec![doc::verbatim(lines[0].to_string())];
    for (i, l) in lines.iter().enumerate().skip(1) {
        parts.push(doc::hardline());
        let cut = if bare_end && i == last {
            l.trim()
        } else {
            l.get(common.min(indent(l))..).unwrap_or("")
        };
        parts.push(doc::verbatim(cut.to_string()));
    }
    Some(doc::concat(parts))
}

/// What the safety check compares of a raw string `reindent` may have rewritten: its lines
/// without their indentation; `None` for any other node.
pub fn value(options: &ResolvedOptions, g: &Grammar, node: &SyntaxNode) -> Option<String> {
    let text = target(options, g, node)?;
    Some(
        text.lines()
            .map(str::trim_start)
            .collect::<Vec<_>>()
            .join("\n"),
    )
}
