//! The lossless lexer: every byte of the input ends up in exactly one token.
//!
//! A word is an `Unquoted` run up to a blank, a parenthesis, `#` or `"`; `\` escapes the next
//! character. A `"` starts a `Quoted` token and a `[` with `=`s and a `[` at the start of a word
//! starts a `Bracket`, wherever the previous token ends: `-DFOO="bar"` is `Unquoted` `Quoted`.
//! An unterminated quoted or bracket text (and comment) ends before the next line that starts a
//! command at column 0, else at the end of the file.
//!
//! concept: none (the tokens of CMake on the lexer base of core_syntax::parse)

use crate::generated::kind as k;
use core_syntax::grammars;
use core_syntax::parse::{self, ParseError, Token};
use core_syntax::strings::StringSyntax;

pub fn lex(src: &str) -> (Vec<Token>, Vec<ParseError>) {
    let mut lx = Lexer {
        lx: parse::Lexer::new(src, src.len() / 4),
        quoted: grammars::CMAKE.get().string_row(k::QUOTED),
    };
    while lx.lx.pos < src.len() {
        lx.next();
    }
    lx.lx.finish(k::EOF)
}

struct Lexer<'a> {
    lx: parse::Lexer<'a>,
    quoted: &'static StringSyntax,
}

impl Lexer<'_> {
    fn next(&mut self) {
        let lx = &mut self.lx;
        if lx.blank(k::WHITESPACE, k::NEWLINE, |c| {
            matches!(c, b' ' | b'\t' | 0x0b | 0x0c)
        }) {
            return;
        }
        let start = lx.pos;
        match lx.at(0) {
            c @ (b'(' | b')') => {
                lx.pos += 1;
                lx.emit(if c == b'(' { k::L_PAREN } else { k::R_PAREN }, start);
            }
            b'#' => match bracket_open(lx.src, start + 1) {
                Some(level) => self.bracket(start + 1, level, k::BRACKET_COMMENT),
                None => {
                    lx.eat_line();
                    lx.emit(k::LINE_COMMENT, start);
                }
            },
            b'"' => {
                let (end, closed) = self.quoted.scan(lx.src, start, false);
                lx.pos = end;
                if !closed {
                    lx.pos = unterminated(lx.src, start);
                    lx.error(start, "unterminated-string", "unterminated quoted argument");
                }
                lx.emit(k::QUOTED, start);
            }
            b'[' if bracket_open(lx.src, start).is_some() => {
                let level = bracket_open(lx.src, start).unwrap_or(0);
                self.bracket(start, level, k::BRACKET);
            }
            _ => {
                unquoted(lx);
                lx.emit(k::UNQUOTED, start);
            }
        }
    }

    /// A bracket text that opens at `open` (`[` and `level` `=`s): argument or comment.
    fn bracket(&mut self, open: usize, level: usize, kind: core_syntax::SyntaxKind) {
        let lx = &mut self.lx;
        let start = if kind == k::BRACKET { open } else { open - 1 };
        let closer = format!("]{}]", "=".repeat(level));
        match parse::raw_end(lx.src.as_bytes(), open + level + 2, closer.as_bytes()) {
            Some(end) => lx.pos = end,
            None => {
                lx.pos = unterminated(lx.src, start);
                lx.error(start, "unterminated-bracket", "unterminated bracket text");
            }
        }
        lx.emit(kind, start);
    }
}

/// The word that starts at the position.
fn unquoted(lx: &mut parse::Lexer<'_>) {
    let b = lx.src.as_bytes();
    let mut i = lx.pos;
    while i < b.len() {
        match b[i] {
            b' ' | b'\t' | 0x0b | 0x0c | b'\n' | b'\r' | b'(' | b')' | b'#' | b'"' => break,
            // The escaped character, a line break (also `\r\n`) included.
            b'\\' if i + 1 < b.len() => {
                i += if b[i + 1..].starts_with(b"\r\n") {
                    3
                } else {
                    2
                }
            }
            _ => i += 1,
        }
    }
    lx.pos = i.max(lx.pos + 1);
}

/// The level of the bracket that opens at `at` (`[`, `=`s, `[`).
fn bracket_open(src: &str, at: usize) -> Option<usize> {
    let rest = src.as_bytes().get(at..)?;
    let level = rest.iter().skip(1).take_while(|&&c| c == b'=').count();
    (rest.first() == Some(&b'[') && rest.get(level + 1) == Some(&b'[')).then_some(level)
}

/// Where the text that opened at `start` and has no end stops: before the next line that starts
/// a command at column 0 (a name and `(`), else at the end of the file.
fn unterminated(src: &str, start: usize) -> usize {
    parse::next_line_where(src, start, |line| {
        let name = line.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'));
        let starts_name = line.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_');
        starts_name
            && name.is_some_and(|n| line[n..].trim_start_matches([' ', '\t']).starts_with('('))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_byte_is_in_a_token() {
        let src = "a(\"x\\\ny\" [==[ ]] ]==] b#c\r\n#[[d]]e\\ f -D=\"g\" \u{e9})";
        let (tokens, errors) = lex(src);
        let joined: String = tokens.iter().map(|t| &src[t.start..t.end]).collect();
        assert_eq!(joined, src);
        assert!(errors.is_empty());
    }
}
