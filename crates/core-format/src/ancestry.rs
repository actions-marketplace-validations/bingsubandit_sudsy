//! The ancestor test of layout rules over a tree of any shape: is a node under a node of one kind
//! set, before one of another set ends the search?
//!
//! concept: ancestor-kind-test
//! what: Whether a node has an ancestor of given kinds below a boundary kind.
//! tools: sqlfluff LT14

use core_syntax::SyntaxKind;

/// Walks up from `from` (the parent of the node asked about): true at the first node whose kind is
/// in `under`; the walk ends after a node whose kind is in `stop`.
pub fn under<N: Copy>(
    from: Option<N>,
    parent: impl Fn(N) -> Option<N>,
    kind: impl Fn(N) -> SyntaxKind,
    under: &[SyntaxKind],
    stop: &[SyntaxKind],
) -> bool {
    let mut at = from;
    while let Some(n) = at {
        let k = kind(n);
        if under.contains(&k) {
            return true;
        }
        at = if stop.contains(&k) { None } else { parent(n) };
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stops_at_the_boundary_kind() {
        let kinds = [SyntaxKind(1), SyntaxKind(2), SyntaxKind(3)];
        // 0 <- 1 <- 2 (parent chain); kind of n is kinds[n].
        let parent = |n: usize| n.checked_sub(1);
        let kind = |n: usize| kinds[n];
        assert!(under(Some(2), parent, kind, &[SyntaxKind(1)], &[]));
        assert!(!under(
            Some(2),
            parent,
            kind,
            &[SyntaxKind(1)],
            &[SyntaxKind(3)]
        ));
        assert!(!under(Some(2), parent, kind, &[SyntaxKind(9)], &[]));
    }
}
