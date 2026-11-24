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
    core_format::rewrite::expression_body(g, tree, text, options, &mut edits);
    core_format::rewrite::redundant_template_braces(g, tree, text, options, &mut edits);
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
    (!edits.is_empty()).then(|| core_format::rewrite::apply_edits(text, edits))
}
