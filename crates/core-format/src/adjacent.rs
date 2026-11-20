//! Neighbours of one kind that run on without a blank line in the source stay together: the
//! kinds are the `keep_together_kinds` option (a fact of the language), the decision is here.
//!
//! concept: keep-together-neighbours
//! what: Two neighbours of one listed kind with no blank line between them in the source get none.
//! tools: Prettier markdown link reference definitions and HTML blocks
//! reads: option:keep_together_kinds

use crate::FormatCtx;
use core_config::opt::KEEP_TOGETHER_KINDS;
use core_syntax::SyntaxKind;

/// The kinds of the `keep_together_kinds` option, resolved once per file.
pub fn kinds(cx: &FormatCtx<'_>) -> Vec<SyntaxKind> {
    cx.options
        .strings(KEEP_TOGETHER_KINDS)
        .filter_map(|n| cx.grammar.kind_by_name(n))
        .collect()
}

/// Do `prev` and `next` stay without a blank line: both of one kind in `kinds`, and no blank line
/// between them in the source (`blank_in_source`)?
pub fn stay_together(
    kinds: &[SyntaxKind],
    prev: SyntaxKind,
    next: SyntaxKind,
    blank_in_source: bool,
) -> bool {
    prev == next && kinds.contains(&next) && !blank_in_source
}
