//! Blanks between two tokens of a line: what the ktlint spacing rules require, else what the
//! source has (no blank stays none, any blanks become one).

use crate::generated::kind as k;
use core_syntax::{SyntaxKind, SyntaxToken};

fn parent(t: &SyntaxToken) -> SyntaxKind {
    t.parent().map_or(k::KT_FILE, |p| p.kind())
}

fn grandparent(t: &SyntaxToken) -> SyntaxKind {
    t.parent()
        .and_then(|p| p.parent())
        .map_or(k::KT_FILE, |p| p.kind())
}

fn is_kw(t: &SyntaxToken, words: &[&str]) -> bool {
    t.kind() == k::KEYWORD && words.contains(&t.text())
}

/// The colon of a supertype list, a type parameter bound or a constructor delegation has a blank
/// on both sides; an annotation target has none; any other has one after it.
fn colon_spaced(colon: &SyntaxToken) -> Option<bool> {
    match parent(colon) {
        k::CLASS
        | k::OBJECT_DECLARATION
        | k::SECONDARY_CONSTRUCTOR
        | k::TYPE_PARAMETER
        | k::TYPE_CONSTRAINT => Some(true),
        k::ANNOTATION | k::ANNOTATION_ENTRY | k::ANNOTATION_TARGET | k::FILE_ANNOTATION_LIST => {
            None
        }
        _ => Some(false),
    }
}

fn is_operator(t: &SyntaxToken) -> bool {
    parent(t) == k::OPERATION_REFERENCE
        && matches!(
            grandparent(t),
            k::BINARY_EXPRESSION | k::IS_EXPRESSION | k::BINARY_WITH_TYPE
        )
}

fn is_range(t: &SyntaxToken) -> bool {
    t.kind() == k::RANGE || t.kind() == k::RANGE_UNTIL
}

/// Should a blank separate `prev` and `next` (on one line)? `None`: as the source has it.
pub(super) fn want(prev: &SyntaxToken, next: &SyntaxToken) -> Option<bool> {
    let (p, n) = (prev.kind(), next.kind());
    if n == k::COMMA || n == k::SEMICOLON {
        return Some(false);
    }
    if p == k::COMMA {
        return Some(!matches!(n, k::RPAR | k::RBRACKET | k::GT));
    }
    if p == k::SEMICOLON && n != k::RBRACE {
        return Some(true);
    }
    if p == k::LPAR || p == k::LBRACKET || p == k::AT {
        return Some(false);
    }
    if n == k::RPAR || n == k::RBRACKET {
        return Some(false);
    }
    if p == k::RPAR && matches!(parent(prev), k::IF | k::WHILE | k::FOR) && n != k::SEMICOLON {
        return Some(true);
    }
    if n == k::COLON {
        return colon_spaced(next).or(Some(false));
    }
    if p == k::COLON {
        return colon_spaced(prev).map(|_| true).or(Some(false));
    }
    if n == k::ARROW || p == k::ARROW {
        return Some(true);
    }
    // Operators.
    if is_operator(next) && !is_range(next) {
        return Some(true);
    }
    if is_operator(prev) {
        return Some(!is_range(prev));
    }
    let sign = |t: &SyntaxToken| matches!(t.text().chars().next(), Some('+' | '-'));
    if sign(prev)
        && sign(next)
        && prev
            .text()
            .ends_with(next.text().chars().next().unwrap_or(' '))
    {
        return Some(true);
    }
    if is_operator(next) {
        return Some(false);
    }
    if parent(prev) == k::OPERATION_REFERENCE && matches!(grandparent(prev), k::PREFIX_EXPRESSION) {
        return Some(false);
    }
    if parent(next) == k::OPERATION_REFERENCE && grandparent(next) == k::POSTFIX_EXPRESSION {
        return Some(false);
    }
    if n == k::EQ || p == k::EQ {
        return Some(true);
    }
    // Member access.
    if matches!(n, k::DOT | k::SAFE_ACCESS) || matches!(p, k::DOT | k::SAFE_ACCESS | k::COLON_COLON)
    {
        return Some(false);
    }
    if n == k::COLON_COLON && matches!(p, k::IDENTIFIER | k::RPAR | k::GT | k::QUEST) {
        return Some(false);
    }
    if n == k::EXCL_EXCL {
        return Some(false);
    }
    if n == k::QUEST && parent(next) == k::NULLABLE_TYPE {
        return Some(false);
    }
    // Braces.
    if p == k::LBRACE || n == k::RBRACE {
        return (p != k::LBRACE || n != k::RBRACE).then_some(true);
    }
    if n == k::LBRACE {
        return Some(true);
    }
    // Labels, spread, annotations.
    if n == k::AT && matches!(parent(next), k::LABEL | k::LABEL_QUALIFIER) {
        return Some(false);
    }
    if p == k::MUL && parent(prev) == k::VALUE_ARGUMENT {
        return Some(false);
    }
    // Keywords.
    if n == k::LPAR && is_kw(prev, &["if", "for", "while", "when", "catch"]) {
        return Some(true);
    }
    if p == k::RBRACE && is_kw(next, &["else", "catch", "finally"]) {
        return Some(true);
    }
    // Parentheses of calls and declarations.
    if n == k::LPAR
        && matches!(
            parent(next),
            k::VALUE_ARGUMENT_LIST | k::VALUE_PARAMETER_LIST
        )
        && grandparent(next) != k::FUNCTION_TYPE
        && matches!(p, k::IDENTIFIER | k::GT)
    {
        return Some(false);
    }
    // Angle brackets.
    if p == k::LT && matches!(parent(prev), k::TYPE_ARGUMENT_LIST | k::TYPE_PARAMETER_LIST) {
        return Some(false);
    }
    if n == k::GT && matches!(parent(next), k::TYPE_ARGUMENT_LIST | k::TYPE_PARAMETER_LIST) {
        return Some(false);
    }
    if n == k::LT && matches!(parent(next), k::TYPE_ARGUMENT_LIST | k::TYPE_PARAMETER_LIST) {
        return Some(is_kw(prev, &["fun", "val", "var"]));
    }
    if p == k::GT
        && parent(prev) == k::TYPE_PARAMETER_LIST
        && matches!(grandparent(prev), k::FUN | k::PROPERTY)
    {
        return Some(true);
    }
    if n == k::LBRACKET && parent(next) == k::INDICES {
        return Some(false);
    }
    None
}
