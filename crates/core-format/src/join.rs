//! Closing tokens of nested nodes that the source writes apart are written together: `A<B<C> >`
//! as `A<B<C>>`. Only the tree decides: both tokens must be of the joined kinds and belong to a
//! node of the kinds named (a nested bracket the parse reads flat is part of the outer node); a
//! `>` that is a comparison or a shift is in another node and stays apart.
//!
//! concept: closer-join
//! what: Closing brackets of nested nodes, written apart, are joined by an option.
//! tools: clang-format Standard (C++11 reads `>>` as two closing template brackets)
//! reads: kinds:close_join close_join_in

use core_syntax::{SyntaxKind, SyntaxNode, SyntaxToken};

/// The edits `(from, to, "")` that remove the blanks between two closing tokens. `joins`: the
/// kinds of the tokens, `inside`: the kinds of the nodes they close.
pub fn closers(
    tree: &SyntaxNode,
    text: &str,
    joins: &[SyntaxKind],
    inside: &[SyntaxKind],
    edits: &mut Vec<(usize, usize, String)>,
) {
    let bytes = text.as_bytes();
    for (at, _) in text.match_indices('>') {
        // (cheap filter: a blank, then the next `>`)
        let rest = &bytes[at + 1..];
        let blanks = rest
            .iter()
            .take_while(|b| matches!(b, b' ' | b'\t'))
            .count();
        if blanks == 0 || rest.get(blanks) != Some(&b'>') {
            continue;
        }
        let Some(first) = u32::try_from(at)
            .ok()
            .and_then(|at| tree.token_at_offset(at.into()).right_biased())
        else {
            continue;
        };
        let Some(second) = first.next_token().and_then(|b| b.next_token()) else {
            continue;
        };
        let gap = first
            .next_token()
            .is_some_and(|b| b.text().trim().is_empty());
        // The second token closes a node that is the first one's node or holds it (a nested
        // bracket the parse reads flat is part of the outer node).
        let nested = first
            .parent()
            .zip(second.parent())
            .is_some_and(|(a, b)| b.text_range().contains_range(a.text_range()));
        let after = bytes.get(at + blanks + 2);
        let is_in = |t: &SyntaxToken| {
            joins.contains(&t.kind()) && t.parent().is_some_and(|p| inside.contains(&p.kind()))
        };
        if gap && nested && is_in(&first) && is_in(&second) && after != Some(&b'=') {
            edits.push((at + 1, at + 1 + blanks, String::new()));
        }
    }
}
