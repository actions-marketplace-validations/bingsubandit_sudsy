//! Angular expressions are TypeScript plus pipes (`value | date: 'short' : tz`, where `|` is a
//! pipe and never a bit-or) and identifiers that TypeScript reserves (`delete`, `switch`). The pipes
//! are split off and each part is formatted by the TypeScript formatter; a pipe in parentheses
//! (`(a | async)?.b`) stands as an identifier while its surroundings are formatted.

/// Words TypeScript reserves that Angular's expression language takes as names.
const RESERVED: &[&str] = &[
    "await", "break", "case", "catch", "class", "const", "continue", "debugger", "default",
    "delete", "do", "enum", "export", "extends", "finally", "for", "function", "import", "new",
    "return", "super", "switch", "throw", "try", "var", "while", "with", "yield",
];

fn is_ident(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'$'
}

/// The text outside strings and template literals: `f(code, index)` for each byte that is, with
/// the index of the byte; strings are skipped.
fn code_bytes(code: &str) -> impl Iterator<Item = (usize, u8)> + '_ {
    let b = code.as_bytes();
    let (mut quote, mut i) = (0u8, 0usize);
    std::iter::from_fn(move || {
        while i < b.len() {
            let (at, c) = (i, b[i]);
            i += 1;
            if quote != 0 {
                if c == b'\\' {
                    i += 1;
                } else if c == quote {
                    quote = 0;
                }
                continue;
            }
            if matches!(c, b'\'' | b'"' | b'`') {
                quote = c;
                continue;
            }
            return Some((at, c));
        }
        None
    })
}

fn lone_pipe(b: &[u8], i: usize) -> bool {
    b[i] == b'|' && b.get(i + 1) != Some(&b'|') && (i == 0 || b[i - 1] != b'|')
}

/// The statements of an event handler: split at the `;` outside brackets and strings.
pub fn split_statements(code: &str) -> Vec<String> {
    let (mut depth, mut start, mut out) = (0i32, 0, Vec::new());
    for (i, c) in code_bytes(code) {
        match c {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b';' if depth == 0 => {
                out.push(code[start..i].trim().to_string());
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(code[start..].trim().to_string());
    out.retain(|p| !p.is_empty());
    out
}

/// `code` formatted by `fmt`, with its pipes; `None` when a part does not format.
pub fn format(code: &str, fmt: &dyn Fn(&str) -> Option<String>) -> Option<String> {
    let (renamed, names) = rename_reserved(code.trim());
    let out = format_pipes(&renamed, fmt, 0)?;
    Some(names.iter().fold(out, |o, (ph, word)| o.replace(ph, word)))
}

/// Reserved words as identifiers `ngkw_word_`, with the pairs to put back.
fn rename_reserved(code: &str) -> (String, Vec<(String, &'static str)>) {
    let b = code.as_bytes();
    let (mut out, mut names, mut last) = (String::new(), Vec::new(), 0);
    let mut i = 0;
    let positions: Vec<(usize, u8)> = code_bytes(code).collect();
    let mut p = 0;
    while p < positions.len() {
        let (at, c) = positions[p];
        if is_ident(c) && !c.is_ascii_digit() && (at == 0 || !is_ident(b[at - 1])) {
            let mut end = at;
            while end < b.len() && is_ident(b[end]) {
                end += 1;
            }
            let word = &code[at..end];
            let after_dot = code[..at].trim_end().ends_with('.');
            if let Some(r) = RESERVED.iter().find(|r| **r == word)
                && !after_dot
            {
                let ph = format!("ngkw_{r}_");
                out.push_str(&code[last..at]);
                out.push_str(&ph);
                names.push((ph, *r));
                last = end;
            }
            while p < positions.len() && positions[p].0 < end {
                p += 1;
            }
            i = end;
            continue;
        }
        p += 1;
    }
    let _ = i;
    out.push_str(&code[last..]);
    (out, names)
}

fn format_pipes(code: &str, fmt: &dyn Fn(&str) -> Option<String>, depth: usize) -> Option<String> {
    let b = code.as_bytes();
    if !code_bytes(code).any(|(i, _)| lone_pipe(b, i)) {
        return fmt(code);
    }
    if depth > 8 {
        return None;
    }
    let (masked, groups) = mask_groups(code)?;
    let mb = masked.as_bytes();
    // Split at the lone pipes outside brackets, then each pipe's arguments at `:` (a `?` takes
    // the `:` of its conditional).
    let (mut depth_b, mut cuts) = (0i32, Vec::new());
    for (i, c) in code_bytes(&masked) {
        match c {
            b'(' | b'[' | b'{' => depth_b += 1,
            b')' | b']' | b'}' => depth_b -= 1,
            b'|' if depth_b == 0 && lone_pipe(mb, i) => cuts.push(i),
            _ => {}
        }
    }
    let mut pieces = Vec::new();
    let mut from = 0;
    for &c in &cuts {
        pieces.push(&masked[from..c]);
        from = c + 1;
    }
    pieces.push(&masked[from..]);
    let mut out = fmt(pieces[0].trim())?;
    for piece in &pieces[1..] {
        let parts = split_arguments(piece);
        let name = parts[0].trim();
        out.push_str(" | ");
        out.push_str(name);
        let mut args = Vec::new();
        for a in &parts[1..] {
            args.push(fmt(a.trim())?);
        }
        if !args.is_empty() {
            out.push_str(": ");
            out.push_str(&args.join(" : "));
        }
    }
    for (ph, inner) in groups {
        let inner = format_pipes(inner.trim(), fmt, depth + 1)?;
        out = out.replace(&ph, &format!("({inner})"));
    }
    Some(out)
}

/// A pipe's name and arguments: split at the `:` outside brackets that no `?` takes.
fn split_arguments(piece: &str) -> Vec<&str> {
    let b = piece.as_bytes();
    let (mut depth, mut open_conditionals, mut from, mut out) = (0i32, 0, 0, Vec::new());
    for (i, c) in code_bytes(piece) {
        match c {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b'?' if depth == 0
                && b.get(i + 1) != Some(&b'.')
                && b.get(i + 1) != Some(&b'?')
                && (i == 0 || b[i - 1] != b'?') =>
            {
                open_conditionals += 1;
            }
            b':' if depth == 0 => {
                if open_conditionals > 0 {
                    open_conditionals -= 1;
                } else {
                    out.push(&piece[from..i]);
                    from = i + 1;
                }
            }
            _ => {}
        }
    }
    out.push(&piece[from..]);
    out
}

/// `code` with each parenthesized group that holds a pipe replaced by an identifier, and the
/// groups' contents; `None` for a pipe in a call, array or object.
fn mask_groups(code: &str) -> Option<(String, Vec<(String, String)>)> {
    let b = code.as_bytes();
    // (start of the group, holds a pipe)
    let mut stack: Vec<(usize, bool)> = Vec::new();
    let mut found: Vec<(usize, usize)> = Vec::new();
    for (i, c) in code_bytes(code) {
        match c {
            b'(' | b'[' | b'{' => stack.push((i, false)),
            b')' | b']' | b'}' => {
                let (start, has) = stack.pop()?;
                if has {
                    let before = code[..start].trim_end().as_bytes().last().copied();
                    let call = b[start] != b'('
                        || before.is_some_and(|c| is_ident(c) || matches!(c, b')' | b']'));
                    if call {
                        return None;
                    }
                    found.push((start, i + 1));
                    if let Some(top) = stack.last_mut() {
                        top.1 = true;
                    }
                }
            }
            b'|' if lone_pipe(b, i) => {
                if let Some(top) = stack.last_mut() {
                    top.1 = true;
                }
            }
            _ => {}
        }
    }
    // Keep the outermost groups only.
    let mut out = String::new();
    let (mut groups, mut at) = (Vec::new(), 0);
    found.sort();
    for (s, e) in found {
        if s < at {
            continue;
        }
        let ph = format!("ngpipe_{}_", groups.len());
        out.push_str(&code[at..s]);
        out.push_str(&ph);
        groups.push((ph, code[s + 1..e - 1].to_string()));
        at = e;
    }
    out.push_str(&code[at..]);
    Some((out, groups))
}
