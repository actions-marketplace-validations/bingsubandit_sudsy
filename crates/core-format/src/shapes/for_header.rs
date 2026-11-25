//! The header of a `for (init; test; update)` statement whose three clauses are all present: the
//! parts break together, each after its `;`, and the parenthesis closes on its own line. A header
//! with an empty clause (`for (;;)`, `for (i = 0; ; i++)`) is not one: it keeps its layout. The
//! caller passes the token kinds; the shape only decides.
//!
//! concept: for-header
//! what: A for header with all three clauses, broken at its semicolons.
//! tools: none (internal)
//! reads: grammar:language

use core_syntax::{SyntaxElement, SyntaxKind};

/// The token kinds of a `for` header.
#[derive(Clone, Copy)]
pub struct ForTokens {
    pub open: SyntaxKind,
    pub semi: SyntaxKind,
    pub close: SyntaxKind,
}

/// Does the header in `els` (the children of the statement) have all three clauses?
pub fn full_for_header(els: &[SyntaxElement], t: ForTokens) -> bool {
    let header: Vec<SyntaxKind> = els
        .iter()
        .map(SyntaxElement::kind)
        .skip_while(|&k| k != t.open)
        .take_while(|&k| k != t.close)
        .collect();
    !header
        .windows(2)
        .any(|w| (w[0] == t.open || w[0] == t.semi) && w[1] == t.semi)
        && header.last().is_some_and(|&k| k != t.semi)
}

/// Is a line break allowed between `prev` and `next`: `prev` is a `;` of a full header and `next`
/// starts a clause?
pub fn breaks_between(
    els: &[SyntaxElement],
    prev: &SyntaxElement,
    next: &SyntaxElement,
    t: ForTokens,
) -> bool {
    prev.kind() == t.semi
        && next.kind() != t.semi
        && next.kind() != t.close
        && full_for_header(els, t)
}

/// The offset of the closing parenthesis from `els[at]`, when that is the `(` of a full header.
pub fn header_close(els: &[SyntaxElement], at: usize, t: ForTokens) -> Option<usize> {
    (els[at].kind() == t.open && full_for_header(els, t))
        .then(|| els[at..].iter().position(|e| e.kind() == t.close))
        .flatten()
}
