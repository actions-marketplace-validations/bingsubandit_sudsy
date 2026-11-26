//! Prototype of a hand-written Groovy lexer for the parser comparison: tokens that tile the source (whitespace, newlines and
//! comments are tokens), GStrings with nested `${ }`, slashy and dollar-slashy strings by the previous token, triple quotes.
//! Measured only: unknown tokens, tiling, bracket balance, speed. Not product code.
use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum K { Ws, Nl, LineComment, BlockComment, Ident, Number, Str, Slashy, Punct, Open, Close, Unknown }

const OPS: &[&str] = &[
    ">>>=", "<<=", ">>=", ">>>", "**=", "<=>", "===", "!==", "...", "?.=", "==~", "?[", "&&=", "||=", "?:=",
    "**", "++", "--", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "==", "!=", "<=", ">=", "&&", "||", "<<", ">>", "->", "?.", "*.", ".&", ".@", "::", "=~", "?:", "..<", "..",
    "+", "-", "*", "/", "%", "&", "|", "^", "~", "!", "<", ">", "=", "?", ":", ",", ";", ".", "@", "\\", "#",
];

fn skip_dq(s: &[u8], mut i: usize, triple: bool) -> usize {
    // after the opening quote(s); returns the index after the closing quote(s) or the end
    while i < s.len() {
        match s[i] {
            b'\\' => i += 2,
            b'$' if s.get(i + 1) == Some(&b'{') => i = skip_expr(s, i + 2),
            b'"' if !triple => return i + 1,
            b'"' if s[i..].starts_with(b"\"\"\"") => return i + 3,
            b'\n' if !triple => return i,
            _ => i += 1,
        }
    }
    s.len()
}

fn skip_expr(s: &[u8], mut i: usize) -> usize {
    let mut depth = 1;
    while i < s.len() && depth > 0 {
        match s[i] {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            b'\'' => { i = skip_sq(s, i + 1, s[i..].starts_with(b"'''")); continue; }
            b'"' => { let t = s[i..].starts_with(b"\"\"\""); i = skip_dq(s, i + if t { 3 } else { 1 }, t); continue; }
            _ => {}
        }
        i += 1;
    }
    i
}

fn skip_sq(s: &[u8], mut i: usize, triple: bool) -> usize {
    if triple { i += 2; }
    while i < s.len() {
        match s[i] {
            b'\\' => i += 2,
            b'\'' if !triple => return i + 1,
            b'\'' if s[i..].starts_with(b"'''") => return i + 3,
            b'\n' if !triple => return i,
            _ => i += 1,
        }
    }
    s.len()
}

pub fn lex(s: &[u8], out: &mut Vec<(K, u32, u32)>) {
    out.clear();
    let (mut i, mut prev) = (0usize, K::Nl);
    let mut prev_text: &[u8] = b"";
    if s.starts_with(b"#!") { let e = s.iter().position(|&b| b == b'\n').unwrap_or(s.len()); out.push((K::LineComment, 0, e as u32)); i = e; }
    while i < s.len() {
        let c = s[i];
        let start = i;
        let kind = match c {
            b' ' | b'\t' | b'\x0c' => { while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\x0c') { i += 1 } K::Ws }
            b'\n' => { i += 1; K::Nl }
            b'\r' => { i += if s.get(i + 1) == Some(&b'\n') { 2 } else { 1 }; K::Nl }
            b'/' if s.get(i + 1) == Some(&b'/') => { while i < s.len() && s[i] != b'\n' && s[i] != b'\r' { i += 1 } K::LineComment }
            b'/' if s.get(i + 1) == Some(&b'*') => { i = s[i + 2..].windows(2).position(|w| w == b"*/").map_or(s.len(), |p| i + 2 + p + 2); K::BlockComment }
            b'\'' => { let t = s[i..].starts_with(b"'''"); i = skip_sq(s, i + 1, t); K::Str }
            b'"' => { let t = s[i..].starts_with(b"\"\"\""); i = skip_dq(s, i + if t { 3 } else { 1 }, t); K::Str }
            b'$' if s.get(i + 1) == Some(&b'/') => { i = s[i + 2..].windows(2).position(|w| w == b"/$").map_or(s.len(), |p| i + 2 + p + 2); K::Slashy }
            b'/' if !matches!(prev, K::Ident | K::Number | K::Str | K::Close | K::Slashy) || (prev == K::Ident && matches!(prev_text, b"return" | b"in" | b"case" | b"assert" | b"throw")) => {
                let mut j = i + 1;
                while j < s.len() && s[j] != b'/' && s[j] != b'\n' { j += if s[j] == b'\\' { 2 } else { 1 } }
                if j < s.len() && s[j] == b'/' && j > i + 1 { i = j + 1; K::Slashy } else { i += 1; K::Punct }
            }
            b'(' | b'[' | b'{' => { i += 1; K::Open }
            b')' | b']' | b'}' => { i += 1; K::Close }
            b'0'..=b'9' => {
                if s[i..].starts_with(b"0x") || s[i..].starts_with(b"0b") { i += 2 }
                while i < s.len() && (s[i].is_ascii_alphanumeric() || s[i] == b'_' || (s[i] == b'.' && s.get(i + 1).is_some_and(|d| d.is_ascii_digit())) || ((s[i] == b'+' || s[i] == b'-') && matches!(s[i - 1], b'e' | b'E') && !s[start..].starts_with(b"0x"))) { i += 1 }
                K::Number
            }
            b'A'..=b'Z' | b'a'..=b'z' | b'_' | b'$' | 0x80..=0xff => {
                while i < s.len() && (s[i].is_ascii_alphanumeric() || matches!(s[i], b'_' | b'$' | 0x80..=0xff)) { i += 1 }
                K::Ident
            }
            _ => match OPS.iter().find(|o| s[i..].starts_with(o.as_bytes())) { Some(o) => { i += o.len(); K::Punct } None => { i += 1; K::Unknown } },
        };
        if !matches!(kind, K::Ws | K::Nl | K::LineComment | K::BlockComment) { prev = kind; prev_text = &s[start..i]; }
        if kind == K::Nl && !matches!(prev, K::Open) { prev = K::Nl; }
        out.push((kind, start as u32, i as u32));
    }
}

pub fn run(files: &[(String, Vec<u8>)], speed: bool) {
    let mut toks = Vec::new();
    if speed {
        let total: usize = files.iter().map(|f| f.1.len()).sum();
        let mut best = f64::MAX;
        for _ in 0..5 {
            let t = Instant::now();
            for (_, s) in files { lex(s, &mut toks); std::hint::black_box(&toks); }
            best = best.min(t.elapsed().as_secs_f64());
        }
        println!("lexer: {} files {} bytes best {:.1} ms {:.1} MB/s", files.len(), total, best * 1e3, total as f64 / 1e6 / best);
        return;
    }
    let (mut unknown_files, mut unbalanced, mut slashy, mut gaps) = (0, 0, 0, 0);
    for (name, s) in files {
        lex(s, &mut toks);
        let mut pos = 0;
        let mut ok = true;
        let mut stack = Vec::new();
        let mut bal = true;
        let mut first_unknown = None;
        for &(k, a, b) in &toks {
            ok &= a as usize == pos;
            pos = b as usize;
            match k {
                K::Open => stack.push(s[a as usize]),
                K::Close => bal &= matches!((stack.pop(), s[a as usize]), (Some(b'('), b')') | (Some(b'['), b']') | (Some(b'{'), b'}')),
                K::Unknown => { first_unknown.get_or_insert(a); }
                K::Slashy => slashy += 1,
                _ => {}
            }
        }
        ok &= pos == s.len();
        bal &= stack.is_empty();
        gaps += !ok as usize;
        unbalanced += !bal as usize;
        if let Some(a) = first_unknown {
            unknown_files += 1;
            println!("unknown\t{name}\t{}", String::from_utf8_lossy(&s[a as usize..(a as usize + 20).min(s.len())]).replace('\n', " "));
        }
        if !bal { println!("unbalanced\t{name}"); }
    }
    println!("lexer: {} files; not tiling {gaps}; with an unknown token {unknown_files}; brackets not balanced {unbalanced}; slashy strings {slashy}", files.len());
}
