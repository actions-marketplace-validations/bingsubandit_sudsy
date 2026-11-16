//! Blank lines between declarations, after the header of a file and at the edges of bodies.

use super::plan::Item;
use super::src::Source;
use crate::generated::kind as k;
use core_syntax::{SyntaxKind, SyntaxNode};

fn is_decl(kind: SyntaxKind) -> bool {
    [
        k::CLASS,
        k::FUN,
        k::PROPERTY,
        k::OBJECT_DECLARATION,
        k::SECONDARY_CONSTRUCTOR,
        k::TYPEALIAS,
        k::CLASS_INITIALIZER,
    ]
    .contains(&kind)
}

fn is_trivia(kind: SyntaxKind) -> bool {
    kind == k::WHITESPACE || kind == k::NEWLINE || kind == k::COMMENT
}

fn has_tokens(node: &SyntaxNode) -> bool {
    node.descendants_with_tokens()
        .any(|e| e.as_token().is_some_and(|t| !is_trivia(t.kind())))
}

/// The outermost node that starts with token `i`.
fn started_by(src: &Source, i: usize) -> Option<SyntaxNode> {
    let tok = &src.toks[i];
    let mut found = None;
    for a in tok.parent_ancestors() {
        let first = a
            .descendants_with_tokens()
            .find_map(|e| e.into_token().filter(|t| !is_trivia(t.kind())));
        if first.as_ref() != Some(tok) {
            break;
        }
        found = Some(a);
    }
    found
}

/// Whether the token ends an annotation of a modifier list.
fn ends_annotation(tok: &core_syntax::SyntaxToken) -> bool {
    tok.parent_ancestors().any(|a| {
        a.kind() == k::ANNOTATION_ENTRY
            && a.parent().is_some_and(|p| p.kind() == k::MODIFIER_LIST)
            && a.last_token().is_some_and(|l| {
                let mut l = l;
                while is_trivia(l.kind()) {
                    match l.prev_token() {
                        Some(p) => l = p,
                        None => break,
                    }
                }
                &l == tok
            })
    })
}

fn prev_node(node: &SyntaxNode) -> Option<SyntaxNode> {
    let mut p = node.prev_sibling();
    while let Some(n) = &p {
        if has_tokens(n) {
            return p;
        }
        p = n.prev_sibling();
    }
    None
}

/// Whether a blank line must separate `node` from the node `prev` before it.
fn bodiless(n: &SyntaxNode) -> bool {
    n.kind() == k::SECONDARY_CONSTRUCTOR && !n.children().any(|c| c.kind() == k::BLOCK)
}

fn needs_blank(node: &SyntaxNode, prev: &SyntaxNode) -> bool {
    let parent = node.parent().map(|p| p.kind());
    if parent == Some(k::KT_FILE)
        && [
            k::PACKAGE_DIRECTIVE,
            k::IMPORT_LIST,
            k::FILE_ANNOTATION_LIST,
        ]
        .contains(&prev.kind())
    {
        return !(prev.kind() == k::FILE_ANNOTATION_LIST && node.kind() == k::PACKAGE_DIRECTIVE);
    }
    match parent {
        Some(p) if p == k::KT_FILE || p == k::CLASS_BODY => {
            is_decl(node.kind())
                && !([k::PROPERTY, k::TYPEALIAS].contains(&node.kind())
                    && prev.kind() == node.kind())
                && !(bodiless(node) && bodiless(prev))
        }
        Some(p) if p == k::BLOCK => is_decl(node.kind()) && node.kind() != k::PROPERTY,
        _ => false,
    }
}

/// Adjusts the line breaks of the gaps in `items`.
#[allow(clippy::needless_range_loop)]
pub(super) fn apply(src: &Source, items: &mut [Vec<Item>]) {
    let n = src.toks.len();
    for i in 1..n {
        let gap = &mut items[i];
        let Some(first_break) = gap.iter().position(|it| matches!(it, Item::Break(_))) else {
            continue;
        };
        let prev = &src.toks[i - 1];
        let next = &src.toks[i];
        let has_comment = gap.iter().any(|it| matches!(it, Item::Comment(..)));
        let body_open = prev.kind() == k::LBRACE
            && prev.parent().is_some_and(|p| {
                p.kind() == k::BLOCK || p.kind() == k::FUNCTION_LITERAL || p.kind() == k::CLASS_BODY
            });
        let leads = matches!(gap.first(), Some(Item::Break(_)));
        if (body_open && leads) || (next.kind() == k::RBRACE && !has_comment) {
            gap[first_break] = Item::Break(1);
            if !has_comment {
                continue;
            }
        }
        if prev.kind() == k::LBRACE {
            continue;
        }
        if !has_comment && ends_annotation(prev) {
            gap[first_break] = Item::Break(1);
            continue;
        }
        let Some(node) = started_by(src, i) else {
            continue;
        };
        let Some(before) = prev_node(&node) else {
            continue;
        };
        if !needs_blank(&node, &before) {
            continue;
        }
        // The blank goes before a comment that leads the declaration.
        let at = if has_comment {
            let kdoc = gap
                .iter()
                .position(|it| matches!(it, Item::Comment(t, _) if t.starts_with("/**")));
            // A comment is only attached to the declaration when no blank line follows it.
            if gap.iter().any(|it| matches!(it, Item::Break(2))) && kdoc.is_none() {
                let last_blank = gap
                    .iter()
                    .rposition(|it| matches!(it, Item::Break(2)))
                    .unwrap_or(0);
                if last_blank + 1 != gap.len() || gap.len() > 1 {
                    continue;
                }
            }
            match kdoc.or_else(|| gap.iter().position(|it| matches!(it, Item::Comment(..)))) {
                Some(c) if c > 0 && matches!(gap[c - 1], Item::Break(_)) => c - 1,
                _ => continue,
            }
        } else {
            first_break
        };
        gap[at] = Item::Break(2);
    }
}
