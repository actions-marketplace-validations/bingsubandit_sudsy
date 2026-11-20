//! What ktlint rewrites rather than lays out, done on the source before it is formatted (the
//! formatter's `rewrite_source`). Each rewrite is a list of edits of the text,
//! so what stays is untouched; the result is parsed again.
//!
//! concept: source-rewrite

use crate::generated::kind as k;
use core_rules::imports::{Compare, Order, sort_edits, words};
use core_syntax::{Grammar, SyntaxNode};

/// The import order of `ktlint_official`: all imports in ASCII order of their path, `java.`,
/// `javax.` and `kotlin.` imports after the others, aliased imports last; no duplicates. A list
/// with a comment is left alone.
static IMPORT_ORDER: std::sync::LazyLock<Order> = std::sync::LazyLock::new(|| Order {
    dedup: true,
    last_prefixes: words(&["java.", "javax.", "kotlin."]),
    ..Order::new(Compare::Ordinal, &["*", "alias"])
});

/// A blank line between a `//` comment and a documentation comment that follows it directly.
fn blank_before_kdoc(tree: &SyntaxNode, text: &str, edits: &mut Vec<(usize, usize, String)>) {
    for t in tree
        .descendants_with_tokens()
        .filter_map(|e| e.into_token())
    {
        let (s, e) = (
            usize::from(t.text_range().start()),
            usize::from(t.text_range().end()),
        );
        if t.kind() == k::COMMENT && text[s..e].starts_with("//") && text[e..].starts_with("\n") {
            let rest = text[e + 1..].trim_start_matches([' ', '\t']);
            if rest.starts_with("/**") {
                edits.push((e, e, "\n".into()));
            }
        }
    }
}

/// `fun f(): T { return x }` becomes `fun f(): T = x` (also `throw`): a body that is one `return` or
/// `throw` without a label and without comments.
fn expression_body(tree: &SyntaxNode, edits: &mut Vec<(usize, usize, String)>) {
    for fun in tree.descendants().filter(|n| n.kind() == k::FUN) {
        let Some(block) = fun.children().find(|n| n.kind() == k::BLOCK) else {
            continue;
        };
        let mut items = block.children();
        let (Some(stmt), None) = (items.next(), items.next()) else {
            continue;
        };
        let labelled = stmt.children().any(|c| c.kind() == k::LABEL_QUALIFIER);
        let value = stmt
            .children()
            .find(|c| !matches!(c.kind(), k::LABEL_QUALIFIER));
        let (Some(value), false) = (value, labelled) else {
            continue;
        };
        let simple = matches!(stmt.kind(), k::RETURN | k::THROW);
        let comment = block
            .descendants_with_tokens()
            .any(|e| e.kind() == k::COMMENT);
        if !simple || comment {
            continue;
        }
        let (b, v) = (block.text_range(), value.text_range());
        let rbrace = block
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == k::RBRACE);
        let Some(end) = rbrace.map(|t| t.text_range().end()) else {
            continue;
        };
        let prefix = if stmt.kind() == k::THROW {
            "= throw "
        } else {
            "= "
        };
        edits.push((b.start().into(), v.start().into(), prefix.into()));
        edits.push((v.end().into(), end.into(), String::new()));
    }
}

/// The source with the rewrites applied, or `None` when there is nothing to rewrite.
pub fn rewrite(
    tree: &SyntaxNode,
    text: &str,
    g: &Grammar,
    options: &core_config::ResolvedOptions,
) -> Option<String> {
    let mut edits: Vec<(usize, usize, String)> = sort_edits(tree, text, g, &IMPORT_ORDER)
        .into_iter()
        .map(|(r, new)| (r.start().into(), r.end().into(), new))
        .collect();
    blank_before_kdoc(tree, text, &mut edits);
    core_format::rewrite::trailing_comma_last_entry(g, tree, text, options, &mut edits);
    expression_body(tree, &mut edits);
    core_format::rewrite::trailing_commas_before(g, tree, text, options, &mut edits);
    core_format::rewrite::redundant_semicolons(
        g,
        tree,
        text,
        &options
            .strings(core_config::opt::SEMICOLON_KEEP_PARENTS)
            .collect::<Vec<_>>(),
        &mut edits,
    );
    if edits.is_empty() {
        return None;
    }
    edits.sort();
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for (s, e, with) in edits {
        out.push_str(&text[at..s]);
        out.push_str(&with);
        at = e;
    }
    out.push_str(&text[at..]);
    Some(out)
}
