//! The lexer: every token of a C or C++ file, read without preprocessing. Comments, whitespace and
//! line continuations are not tokens here (`trivia` finds them again between the tokens); what
//! is left is classified by its first bytes only, so any bytes lex and nothing is an error but a
//! comment or raw string that never ends.
//!
//! A token carries the number of the preprocessor directive it belongs to (0 for none): a
//! directive is a `#` that starts a line and everything to the end of the line, over line
//! continuations and block comments.
//!
//! concept: new source-lexer
//! what: Lexer: the source as tokens with byte spans for a hand-written parser.

use crate::Dialect;
use crate::generated::kind as k;
use core_syntax::SyntaxKind;

pub struct LTok {
    pub kind: SyntaxKind,
    pub start: u32,
    pub end: u32,
    /// The directive the token belongs to, counted from 1; 0 outside directives.
    pub dir: u32,
}

pub struct Lexed {
    pub toks: Vec<LTok>,
    /// Start and length of each comment or raw string that never ends.
    pub errors: Vec<(u32, u32, &'static str)>,
}

fn is_keyword(dialect: Dialect, s: &str) -> bool {
    match dialect {
        Dialect::C | Dialect::CFile => matches!(
            s,
            "alignas"
                | "alignof"
                | "auto"
                | "bool"
                | "break"
                | "case"
                | "char"
                | "const"
                | "constexpr"
                | "continue"
                | "default"
                | "do"
                | "double"
                | "else"
                | "enum"
                | "extern"
                | "false"
                | "float"
                | "for"
                | "goto"
                | "if"
                | "inline"
                | "int"
                | "long"
                | "nullptr"
                | "register"
                | "restrict"
                | "return"
                | "short"
                | "signed"
                | "sizeof"
                | "static"
                | "static_assert"
                | "struct"
                | "switch"
                | "thread_local"
                | "true"
                | "typedef"
                | "typeof"
                | "typeof_unqual"
                | "union"
                | "unsigned"
                | "void"
                | "volatile"
                | "while"
                | "_Alignas"
                | "_Alignof"
                | "_Atomic"
                | "_Bool"
                | "_Complex"
                | "_Generic"
                | "_Imaginary"
                | "_Noreturn"
                | "_Static_assert"
                | "_Thread_local"
        ),
        Dialect::Cpp => matches!(
            s,
            "alignas"
                | "alignof"
                | "and"
                | "and_eq"
                | "asm"
                | "auto"
                | "bitand"
                | "bitor"
                | "bool"
                | "break"
                | "case"
                | "catch"
                | "char"
                | "char8_t"
                | "char16_t"
                | "char32_t"
                | "class"
                | "compl"
                | "concept"
                | "const"
                | "const_cast"
                | "consteval"
                | "constexpr"
                | "constinit"
                | "continue"
                | "co_await"
                | "co_return"
                | "co_yield"
                | "decltype"
                | "default"
                | "delete"
                | "do"
                | "double"
                | "dynamic_cast"
                | "else"
                | "enum"
                | "explicit"
                | "export"
                | "extern"
                | "false"
                | "float"
                | "for"
                | "friend"
                | "goto"
                | "if"
                | "inline"
                | "int"
                | "long"
                | "mutable"
                | "namespace"
                | "new"
                | "noexcept"
                | "not"
                | "not_eq"
                | "nullptr"
                | "operator"
                | "or"
                | "or_eq"
                | "private"
                | "protected"
                | "public"
                | "register"
                | "reinterpret_cast"
                | "requires"
                | "return"
                | "short"
                | "signed"
                | "sizeof"
                | "static"
                | "static_assert"
                | "static_cast"
                | "struct"
                | "switch"
                | "template"
                | "this"
                | "thread_local"
                | "throw"
                | "true"
                | "try"
                | "typedef"
                | "typeid"
                | "typename"
                | "union"
                | "unsigned"
                | "using"
                | "virtual"
                | "void"
                | "volatile"
                | "wchar_t"
                | "while"
                | "xor"
                | "xor_eq"
        ),
    }
}

fn ident_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'$' || c >= 0x80
}

/// The punctuator at the start of `b` (longest first) and its length.
fn punct(b: &[u8]) -> Option<(SyntaxKind, usize)> {
    let at = |i: usize| b.get(i).copied().unwrap_or(0);
    let (c, d, e) = (at(0), at(1), at(2));
    let found = match (c, d, e) {
        (b'.', b'.', b'.') => (k::ELLIPSIS, 3),
        (b'-', b'>', b'*') => (k::ARROW_STAR, 3),
        (b'<', b'<', b'=') => (k::LESS_LESS_EQ, 3),
        (b'>', b'>', b'=') => (k::GREATER_GREATER_EQ, 3),
        (b'<', b'=', b'>') => (k::SPACESHIP, 3),
        (b':', b':', _) => (k::COLON_COLON, 2),
        (b'-', b'>', _) => (k::ARROW, 2),
        (b'.', b'*', _) => (k::DOT_STAR, 2),
        (b'+', b'=', _) => (k::PLUS_EQ, 2),
        (b'-', b'=', _) => (k::MINUS_EQ, 2),
        (b'*', b'=', _) => (k::STAR_EQ, 2),
        (b'/', b'=', _) => (k::SLASH_EQ, 2),
        (b'%', b'=', _) => (k::PERCENT_EQ, 2),
        (b'^', b'=', _) => (k::CARET_EQ, 2),
        (b'&', b'=', _) => (k::AMP_EQ, 2),
        (b'|', b'=', _) => (k::PIPE_EQ, 2),
        (b'<', b'<', _) => (k::LESS_LESS, 2),
        (b'>', b'>', _) => (k::GREATER_GREATER, 2),
        (b'=', b'=', _) => (k::EQ_EQ, 2),
        (b'!', b'=', _) => (k::BANG_EQ, 2),
        (b'<', b'=', _) => (k::LESS_EQ, 2),
        (b'>', b'=', _) => (k::GREATER_EQ, 2),
        (b'&', b'&', _) => (k::AMP_AMP, 2),
        (b'|', b'|', _) => (k::PIPE_PIPE, 2),
        (b'+', b'+', _) => (k::PLUS_PLUS, 2),
        (b'-', b'-', _) => (k::MINUS_MINUS, 2),
        (b'#', b'#', _) => (k::HASH_HASH, 2),
        (b'{', ..) => (k::LBRACE, 1),
        (b'}', ..) => (k::RBRACE, 1),
        (b'(', ..) => (k::LPAR, 1),
        (b')', ..) => (k::RPAR, 1),
        (b'[', ..) => (k::LSQB, 1),
        (b']', ..) => (k::RSQB, 1),
        (b';', ..) => (k::SEMI, 1),
        (b',', ..) => (k::COMMA, 1),
        (b':', ..) => (k::COLON, 1),
        (b'.', ..) => (k::DOT, 1),
        (b'?', ..) => (k::QUESTION, 1),
        (b'+', ..) => (k::PLUS, 1),
        (b'-', ..) => (k::MINUS, 1),
        (b'*', ..) => (k::STAR, 1),
        (b'/', ..) => (k::SLASH, 1),
        (b'%', ..) => (k::PERCENT, 1),
        (b'^', ..) => (k::CARET, 1),
        (b'&', ..) => (k::AMP, 1),
        (b'|', ..) => (k::PIPE, 1),
        (b'~', ..) => (k::TILDE, 1),
        (b'!', ..) => (k::BANG, 1),
        (b'=', ..) => (k::EQUAL, 1),
        (b'<', ..) => (k::LESS, 1),
        (b'>', ..) => (k::GREATER, 1),
        (b'#', ..) => (k::HASH, 1),
        _ => return None,
    };
    Some(found)
}

/// The length of the line break (`\n` or `\r\n`) at the start of `b`, 0 when there is none.
pub fn line_break(b: &[u8]) -> usize {
    match b {
        [b'\n', ..] => 1,
        [b'\r', b'\n', ..] => 2,
        _ => 0,
    }
}

/// The length of the `//` comment at the start of `b`: to the end of the line, over a
/// backslash that ends a line.
pub fn line_comment_len(b: &[u8]) -> usize {
    let mut i = 2;
    while i < b.len() && b[i] != b'\n' {
        if b[i] == b'\\' {
            let n = line_break(&b[i + 1..]);
            if n > 0 {
                i += 1 + n;
                continue;
            }
        }
        i += 1;
    }
    i
}

/// The length of the `/* */` comment at the start of `b` and whether it ends.
pub fn block_comment_len(b: &[u8]) -> (usize, bool) {
    match b[2..].windows(2).position(|w| w == b"*/") {
        Some(i) => (i + 4, true),
        None => (b.len(), false),
    }
}

/// The length of the string or character literal that starts at `src[i]` (a quote), a suffix
/// included unless the file is C (`"a"B` is a string and a macro); it ends at the line's end when
/// the closing quote is missing.
fn quoted_len(src: &str, i: usize, dialect: Dialect) -> usize {
    let row = (core_syntax::grammars::CPP.get())
        .string_rows(k::STRING)
        .next()
        .expect("the C++ grammar has a string row");
    let (end, closed) = row.scan(src, i, false);
    let b = src.as_bytes();
    let suffix = if closed && dialect != Dialect::CFile {
        b[end..].iter().take_while(|&&c| ident_byte(c)).count()
    } else {
        0
    };
    end - i + suffix
}

/// The length of the raw string literal whose `R"` starts at `b[0..2]`, a suffix included, and
/// whether it ends.
fn raw_len(b: &[u8]) -> (usize, bool) {
    let open = b[2..].iter().take(17).position(|&c| c == b'(');
    let Some(open) = open else {
        return (0, true);
    };
    let delim = &b[2..2 + open];
    if delim
        .iter()
        .any(|&c| matches!(c, b' ' | b')' | b'\\' | b'\t' | b'\n'))
    {
        return (0, true);
    }
    let body = 3 + open;
    let mut i = body;
    while i < b.len() {
        if b[i] == b')'
            && b[i + 1..].starts_with(delim)
            && b.get(i + 1 + delim.len()) == Some(&b'"')
        {
            let end = i + delim.len() + 2;
            return (
                end + b[end..].iter().take_while(|&&c| ident_byte(c)).count(),
                true,
            );
        }
        i += 1;
    }
    (b.len(), false)
}

fn number_len(b: &[u8]) -> usize {
    let mut i = 1;
    while i < b.len() {
        let d = b[i];
        if matches!(d, b'e' | b'E' | b'p' | b'P') && matches!(b.get(i + 1), Some(b'+' | b'-')) {
            i += 2;
        } else if d.is_ascii_alphanumeric() || d == b'_' || d == b'.' {
            i += 1;
        } else if d == b'\'' && b.get(i + 1).is_some_and(u8::is_ascii_alphanumeric) {
            i += 2;
        } else {
            break;
        }
    }
    i
}

/// The length of a string prefix (`u8`, `u`, `U`, `L`, each with an optional `R`) that is
/// directly followed by a quote, and whether it is a raw one.
fn string_prefix(b: &[u8]) -> Option<(usize, bool)> {
    let mut i = match b {
        [b'u', b'8', ..] => 2,
        [b'u' | b'U' | b'L', ..] => 1,
        _ => 0,
    };
    let raw = b.get(i) == Some(&b'R');
    i += usize::from(raw);
    match b.get(i) {
        Some(b'"') => Some((i, raw)),
        Some(b'\'') if !raw && i > 0 => Some((i, false)),
        _ => None,
    }
}

pub fn lex(src: &str, dialect: Dialect) -> Lexed {
    let b = src.as_bytes();
    let n = b.len();
    let mut out = Lexed {
        toks: Vec::with_capacity(n / 5),
        errors: Vec::new(),
    };
    let (mut i, mut line_start, mut dir, mut dirs) = (0, true, 0u32, 0u32);
    // After `#include`: the next `<` starts a header name.
    let mut header = false;
    if b.starts_with("\u{feff}".as_bytes()) {
        i = 3;
    }
    macro_rules! tok {
        ($kind:expr, $len:expr) => {{
            let end = i + $len;
            out.toks.push(LTok {
                kind: $kind,
                start: i as u32,
                end: end as u32,
                dir,
            });
            i = end;
        }};
    }
    while i < n {
        let c = b[i];
        match c {
            b'\n' => {
                (i, line_start, dir, header) = (i + 1, true, 0, false);
                continue;
            }
            b' ' | b'\t' | b'\r' | 0x0b | 0x0c => {
                i += 1;
                continue;
            }
            b'\\' if line_break(&b[i + 1..]) > 0 => {
                i += 1 + line_break(&b[i + 1..]);
                continue;
            }
            b'/' if b.get(i + 1) == Some(&b'/') => {
                i += line_comment_len(&b[i..]);
                continue;
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                let (len, closed) = block_comment_len(&b[i..]);
                if !closed {
                    out.errors
                        .push((i as u32, len as u32, "unterminated comment"));
                }
                i += len;
                continue;
            }
            b'#' if line_start => {
                dirs += 1;
                dir = dirs;
                line_start = false;
                header = false;
                tok!(k::HASH, 1);
                continue;
            }
            _ => {}
        }
        line_start = false;
        let was_header = std::mem::take(&mut header);
        if was_header
            && c == b'<'
            && let Some(end) = b[i..].iter().position(|&c| c == b'>' || c == b'\n')
            && b[i + end] == b'>'
        {
            tok!(k::HEADER_NAME, end + 1);
            continue;
        }
        if let Some((p, raw)) = string_prefix(&b[i..]) {
            if raw {
                let (len, closed) = raw_len(&b[i + p - 1..]);
                if len > 0 {
                    if !closed {
                        out.errors
                            .push((i as u32, (n - i) as u32, "unterminated raw string"));
                    }
                    tok!(k::STRING, p - 1 + len);
                    continue;
                }
            } else {
                let kind = if b[i + p] == b'"' { k::STRING } else { k::CHAR };
                tok!(kind, p + quoted_len(src, i + p, dialect));
                continue;
            }
        }
        if c == b'"' || c == b'\'' {
            tok!(
                if c == b'"' { k::STRING } else { k::CHAR },
                quoted_len(src, i, dialect)
            );
        } else if ident_byte(c) && !c.is_ascii_digit() {
            let len = b[i..].iter().take_while(|&&c| ident_byte(c)).count();
            let text = &src[i..i + len];
            // `src` is valid UTF-8 and the run ends before an ASCII byte or the end.
            let kind = if is_keyword(dialect, text) {
                k::KEYWORD
            } else {
                k::IDENT
            };
            header = dir > 0
                && matches!(text, "include" | "include_next" | "import")
                && out.toks.last().is_some_and(|t| t.kind == k::HASH);
            tok!(kind, len);
        } else if c.is_ascii_digit() || (c == b'.' && b.get(i + 1).is_some_and(u8::is_ascii_digit))
        {
            tok!(k::NUMBER, number_len(&b[i..]));
        } else if let Some((kind, len)) = punct(&b[i..]) {
            tok!(kind, len);
        } else {
            tok!(k::OTHER, 1);
        }
    }
    out
}
