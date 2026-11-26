//! A lossless parser of CMake files with error recovery: a flat list of commands, each with its
//! arguments. Blocks (`if` ... `endif`) are not nodes: the shared pairing of `core_syntax::block_pair`
//! reads the command names in order and reports an unpaired block on its own command.
//!
//! Recovery: tokens that are no command are one `ERROR` node up to the next `name(`. A command
//! whose `(` is never closed ends before the next line that starts a command at column 0 (or at
//! the end of the file), so a deleted `)` damages one command.
//!
//! concept: none (the productions of CMake on the parser kit of core_syntax::parse)

use crate::blocks::BLOCKS;
use crate::generated::kind as k;
use crate::lexer::lex;
use core_syntax::SyntaxKind;
use core_syntax::block_pair::Pairing;
use core_syntax::parse::{self, MAX_DEPTH, Parsed};
use provider_api::{ParseResult, SourceFile};

const MAX_GROUPS: usize = MAX_DEPTH / 2;

pub fn parse_file(file: &SourceFile) -> ParseResult {
    let Parsed { tree, errors } = parse(&file.text);
    ParseResult { tree, errors }
}

pub fn parse(src: &str) -> Parsed {
    let mut p = parse::Parser::new(src, lex(src), k::TRIVIA_MASK, k::EOF);
    let closers = p.pairs(k::L_PAREN, k::R_PAREN);
    let mut pairing = Pairing::new(&BLOCKS);
    p.start(k::FILE);
    while !p.at_eof() {
        if starts_command(&p) {
            command(&mut p, &closers, &mut pairing);
        } else {
            stray(&mut p);
        }
    }
    p.bump(); // Eof
    p.finish();
    for f in pairing.finish() {
        p.error_at(f.at as usize, f.fault.code(), f.fault.message());
    }
    p.done()
}

/// At a name followed by `(`.
fn starts_command(p: &parse::Parser<'_>) -> bool {
    p.cur() == k::UNQUOTED && p.kind_at(p.pos + 1) == k::L_PAREN
}

/// Tokens that are no command, up to the next command.
fn stray(p: &mut parse::Parser<'_>) {
    p.error("expected-command", "expected a command: a name and `(`");
    p.start(SyntaxKind::ERROR);
    p.bump();
    while !p.at_eof() && !starts_command(p) {
        p.bump();
    }
    p.finish();
}

fn command(p: &mut parse::Parser<'_>, closers: &[u32], pairing: &mut Pairing) {
    p.start(k::COMMAND);
    pairing.word(p.text_at(p.pos), p.pos as u32);
    p.bump_as(k::NAME);
    let open = p.pos;
    let limit = limit(p, closers, open);
    p.bump(); // (
    arguments(p, limit);
    p.close_group(&GROUP, open);
    p.finish();
}

/// Where the arguments of the command whose `(` is the real token `open` stop at most: the end
/// of the file when the parentheses are paired, else the next command at column 0.
fn limit(p: &parse::Parser<'_>, closers: &[u32], open: usize) -> usize {
    if closers[open] != u32::MAX {
        return p.eof_index();
    }
    let after = |i: usize| p.kind_at(i) == k::UNQUOTED && p.kind_at(i + 1) == k::L_PAREN;
    (open + 1..p.eof_index())
        .find(|&i| after(i) && p.at_line_start(i))
        .unwrap_or(p.eof_index())
}

/// The arguments and groups up to `)` or `limit`.
fn arguments(p: &mut parse::Parser<'_>, limit: usize) {
    p.start(k::ARGUMENT_LIST);
    while p.pos < limit {
        match p.cur() {
            k::R_PAREN | k::EOF => break,
            k::L_PAREN => group(p, limit),
            _ => argument(p),
        }
    }
    p.finish();
}

fn group(p: &mut parse::Parser<'_>, limit: usize) {
    let Some(open) = p.group_start(&GROUP, limit) else {
        return;
    };
    arguments(p, limit);
    p.group_end(&GROUP, open);
}

/// The words that touch one another, with no blank between.
fn argument(p: &mut parse::Parser<'_>) {
    p.start(k::ARGUMENT);
    loop {
        let t = p.tok(p.pos);
        // A quoted or bracket text that ends at a line break had no end.
        let open_ended = t.kind != k::UNQUOTED && p.src[..t.end].ends_with(['\n', '\r']);
        p.bump();
        let next = p.touches_prev();
        if open_ended || !(next && matches!(p.cur(), k::UNQUOTED | k::QUOTED | k::BRACKET)) {
            break;
        }
    }
    p.finish();
}

/// Each paren group is two levels of the tree.
const GROUP: parse::Group = parse::Group::parens(
    k::PAREN_GROUP,
    (k::L_PAREN, k::R_PAREN),
    "parentheses",
    MAX_GROUPS,
    "unclosed-paren",
);
