//! Neighbours that run on without a blank line in the source stay together: the pairs are the
//! `keep_together_kinds` option (a fact of the language: `A` is the pair A, A; `A>B` the pair A, B), the decision is here.
//!
//! concept: keep-together-neighbours
//! what: Two neighbours of a listed kind pair with no blank line between them in the source get none.
//! tools: Prettier markdown link reference definitions, HTML blocks and an HTML block after a paragraph
//! reads: option:keep_together_kinds

use crate::FormatCtx;
use core_config::opt::KEEP_TOGETHER_KINDS;
use core_syntax::SyntaxKind;

/// The pairs of the `keep_together_kinds` option, resolved once per file.
pub fn kinds(cx: &FormatCtx<'_>) -> Vec<(SyntaxKind, SyntaxKind)> {
    let kind = |n: &str| cx.grammar.kind_by_name(n);
    cx.options
        .strings(KEEP_TOGETHER_KINDS)
        .filter_map(|n| {
            let (a, b) = n.split_once('>').unwrap_or((n, n));
            Some((kind(a)?, kind(b)?))
        })
        .collect()
}

/// Do `prev` and `next` stay without a blank line: a pair of `pairs`, and no blank line
/// between them in the source (`blank_in_source`)?
pub fn stay_together(
    pairs: &[(SyntaxKind, SyntaxKind)],
    prev: SyntaxKind,
    next: SyntaxKind,
    blank_in_source: bool,
) -> bool {
    !blank_in_source && pairs.contains(&(prev, next))
}
