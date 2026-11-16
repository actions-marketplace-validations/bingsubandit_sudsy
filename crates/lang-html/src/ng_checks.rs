//! Text checks of Angular template expressions for the `template` family (`template.rs`): they read
//! the code of a binding, an interpolation or a block head as text, skipping string literals.

/// The code of `text` outside string literals, as `(byte index, char)`.
fn code(text: &str) -> impl Iterator<Item = (usize, char)> + '_ {
    let mut quote = None::<char>;
    let mut escaped = false;
    text.char_indices().filter(move |&(_, c)| {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == q {
                quote = None;
            }
            false
        } else if matches!(c, '"' | '\'' | '`') {
            quote = Some(c);
            false
        } else {
            true
        }
    })
}

/// The byte offsets of the `==` and `!=` operators (not `===`, `!==`).
pub(crate) fn loose_equality(text: &str) -> Vec<usize> {
    let b = text.as_bytes();
    code(text)
        .filter(|&(i, c)| {
            matches!(c, '=' | '!')
                && b.get(i + 1) == Some(&b'=')
                && b.get(i + 2) != Some(&b'=')
                && !(c == '=' && i > 0 && matches!(b[i - 1], b'=' | b'!' | b'<' | b'>'))
        })
        .map(|(i, _)| i)
        .collect()
}

/// Is there a pipe to `async` (not `||`) among the code of `text`?
fn has_async_pipe(text: &str) -> bool {
    let b = text.as_bytes();
    code(text).any(|(i, c)| {
        c == '|'
            && b.get(i + 1) != Some(&b'|')
            && (i == 0 || b[i - 1] != b'|')
            && text[i + 1..]
                .trim_start()
                .strip_prefix("async")
                .is_some_and(|r| !r.starts_with(|c: char| c.is_alphanumeric() || c == '_'))
    })
}

/// The offsets of `!(x | async)` and `(x | async) === false`.
pub(crate) fn negated_async(text: &str) -> Vec<usize> {
    let mut out = Vec::new();
    let mut depth = Vec::<usize>::new();
    for (i, c) in code(text) {
        match c {
            '(' => depth.push(i),
            ')' => {
                let Some(open) = depth.pop() else { continue };
                if !has_async_pipe(&text[open + 1..i]) {
                    continue;
                }
                let before = text[..open].trim_end();
                let after = text[i + 1..].trim_start();
                let compared = ["===", "=="].iter().any(|op| {
                    after
                        .strip_prefix(op)
                        .map(str::trim_start)
                        .and_then(|r| r.strip_prefix("false"))
                        .is_some_and(|r| !r.starts_with(|c: char| c.is_alphanumeric() || c == '_'))
                });
                if (before.ends_with('!') && !before.ends_with("!=")) || compared {
                    out.push(open);
                }
            }
            _ => {}
        }
    }
    out
}

/// Does a `@for (...)` head lack its `track` expression?
pub(crate) fn for_without_track(head: &str) -> bool {
    let Some(params) = head.strip_prefix("@for") else {
        return false;
    };
    !params.split(';').any(|p| {
        p.trim_start()
            .strip_prefix("track")
            .is_some_and(|r| r.starts_with(char::is_whitespace))
    })
}

/// The code of a binding attribute value: `[x]`, `(x)`, `*x`, `bind-x`, `on-x` and `bindon-x`.
pub(crate) fn is_binding(name: &str) -> bool {
    name.starts_with(['[', '(', '*'])
        || name.starts_with("bind-")
        || name.starts_with("on-")
        || name.starts_with("bindon-")
}

/// `([x])`: the banana is in the box.
pub(crate) fn banana_in_box(name: &str) -> bool {
    name.len() > 4 && name.starts_with("([") && name.ends_with("])")
}
