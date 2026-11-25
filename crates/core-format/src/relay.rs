//! The text of a line-oriented construct that the source splits with a trailing `\` (a
//! preprocessor directive): blanks outside literals and comments become one, and the backslashes of
//! the lines stand in one column (at the line width, one past the longest line, or one blank after
//! each line).
//!
//! concept: escaped-newline-lines
//! what: A directive's blanks collapse and its trailing backslashes align in a column.
//! tools: clang-format AlignEscapedNewlines

/// Where the backslashes of the lines of a text stand: the kind of alignment, the column of
/// `Align::Right`, and the columns of a tab in the lines.
#[derive(Clone, Copy, Debug)]
pub struct Place {
    pub align: Align,
    pub width: usize,
    pub tab: usize,
}

impl Place {
    /// One blank before each backslash.
    pub const NONE: Place = Place {
        align: Align::None,
        width: 0,
        tab: 8,
    };
}

/// Where the backslashes of the lines of a text stand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    /// In the column `width` (a line that is longer has one blank before its backslash).
    Right,
    /// One column past the longest line.
    Left,
    /// One blank after each line.
    None,
}

impl Align {
    pub fn parse(value: &str) -> Align {
        match value {
            "right" => Align::Right,
            "left" => Align::Left,
            _ => Align::None,
        }
    }
}

/// One line with its runs of blanks outside literals and comments read as one blank, and where
/// the comment that ends it starts (if the line has one after code). `in_block` carries an open
/// block comment, `quote` an open literal (a string continued with a backslash) from line to line.
fn collapse(line: &str, in_block: &mut bool, quote: &mut Option<char>) -> (String, Option<usize>) {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    let mut blank = false;
    let (mut open, mut trailing) = (None, None);
    while let Some(c) = chars.next() {
        if *in_block {
            out.push(c);
            if c == '*' && chars.peek() == Some(&'/') {
                out.extend(chars.next());
                *in_block = false;
                trailing = open;
            }
        } else if let Some(q) = *quote {
            out.push(c);
            if c == '\\' {
                out.extend(chars.next());
            } else if c == q {
                *quote = None;
            }
        } else if c == ' ' || c == '\t' {
            blank = true;
        } else {
            if blank && !out.is_empty() {
                out.push(' ');
            }
            blank = false;
            trailing = None;
            let at = out.len();
            out.push(c);
            match c {
                '"' | '\'' => *quote = Some(c),
                '/' if chars.peek() == Some(&'/') => {
                    out.extend(chars.by_ref());
                    trailing = Some(at);
                    break;
                }
                '/' if chars.peek() == Some(&'*') => {
                    out.extend(chars.next());
                    *in_block = true;
                    open = Some(at);
                }
                _ => {}
            }
        }
    }
    let kept = out.trim_end().len();
    out.truncate(kept);
    let trailing = trailing.filter(|&at| !out[..at].trim_end().is_empty() && at < out.len());
    (out, trailing)
}

/// The blanks in front of a trailing comment: `gap` of them and, when `aligned`, the mark that
/// the comments of consecutive lines are aligned at ([`crate::comments::align_marked_gaps`]).
#[derive(Clone, Copy, Debug)]
pub struct CommentGap {
    pub gap: usize,
    pub aligned: bool,
}

/// `text` with every line collapsed (its indentation kept) and its trailing backslashes aligned.
pub fn relay(text: &str, place: Place, comment: Option<CommentGap>) -> String {
    // A line with no run of blanks, tab, comment, backslash or quote has nothing to collapse.
    if !text.contains(['\t', '\\', '/', '"', '\'', '\n']) && !text.contains("  ") {
        return text.to_string();
    }
    let mut in_block = false;
    let mut quote = None;
    // (the line, whether its backslash is aligned: a line inside a block comment stays as written)
    let mut lines: Vec<(String, bool)> = Vec::new();
    for (i, line) in text.split('\n').enumerate() {
        let line = line.trim_end_matches('\r');
        let (body, escaped) = match line.trim_end().strip_suffix('\\') {
            Some(b) => (b, true),
            None => (line, false),
        };
        let started_in_block = in_block || quote.is_some();
        // The inside of a block comment and a blank line are not re-indented.
        let raw = in_block || body.trim().is_empty();
        let indent = if i == 0 || raw {
            ""
        } else {
            &body[..body.len() - body.trim_start_matches([' ', '\t']).len()]
        };
        let text = if body.trim().is_empty() {
            String::new()
        } else if in_block {
            let mut rest = in_block;
            let (kept, _) = collapse(body.trim_start_matches([' ', '\t']), &mut rest, &mut quote);
            let lead = &body[..body.len() - body.trim_start_matches([' ', '\t']).len()];
            in_block = rest;
            format!("{lead}{kept}")
        } else {
            let (kept, trailing) = collapse(body, &mut in_block, &mut quote);
            match (trailing, comment) {
                (Some(at), Some(c)) => {
                    let mark = if c.aligned && !escaped { "\u{1}" } else { "" };
                    format!(
                        "{}{}{mark}{}",
                        kept[..at].trim_end(),
                        " ".repeat(c.gap),
                        &kept[at..]
                    )
                }
                _ => kept,
            }
        };
        if started_in_block || in_block || quote.is_some() {
            lines.push((line.to_string(), false));
            continue;
        }
        lines.push((format!("{indent}{text}"), escaped));
    }
    align_escapes(&lines, place)
}

/// The lines joined, the backslash of each line that has one in its column.
fn align_escapes(lines: &[(String, bool)], place: Place) -> String {
    let Place { align, width, tab } = place;
    let widest = lines
        .iter()
        .filter(|l| l.1)
        .map(|l| columns(&l.0, tab))
        .max()
        .unwrap_or(0);
    let column = match align {
        Align::Right => width,
        Align::Left => widest + 2,
        Align::None => 0,
    };
    let mut out = String::with_capacity(lines.iter().map(|l| l.0.len() + 4).sum());
    for (i, (line, escaped)) in lines.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(line);
        if *escaped {
            let len = columns(line, tab);
            let pad = if len + 2 <= column {
                column - 1 - len
            } else {
                1
            };
            out.extend(std::iter::repeat_n(' ', pad));
            out.push('\\');
        }
    }
    out
}

/// The lines of a text that is laid out already (a macro body printed by the formatter): every line
/// but the last gets a backslash, in the column `align` says.
pub fn escape_lines(text: &str, place: Place) -> String {
    let count = text.split('\n').count();
    let lines: Vec<(String, bool)> = (text.split('\n'))
        .enumerate()
        .map(|(i, l)| (l.trim_end().to_string(), i + 1 < count))
        .collect();
    align_escapes(&lines, place)
}

/// The columns a line takes: a tab goes to the next multiple of `tab`.
fn columns(line: &str, tab: usize) -> usize {
    line.chars().fold(0, |col, c| match c {
        '\t' => (col / tab.max(1) + 1) * tab.max(1),
        _ => col + 1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn place(align: Align, width: usize) -> Place {
        Place {
            align,
            width,
            tab: 8,
        }
    }

    #[test]
    fn collapses_blanks_and_aligns_backslashes() {
        let text = "#define  A(x)\t\\\n    foo(x,   \"a  b\")   \\\n  bar";
        assert_eq!(
            relay(text, place(Align::Left, 80), None),
            "#define A(x)       \\\n    foo(x, \"a  b\") \\\n  bar"
        );
        assert_eq!(
            relay("#define A  1 // c  d", place(Align::Right, 80), None),
            "#define A 1 // c  d"
        );
        let right = relay(text, place(Align::Right, 20), None);
        assert!(
            right.lines().take(2).all(|l| l.chars().count() == 20),
            "{right}"
        );
    }

    #[test]
    fn a_tab_in_a_line_takes_the_columns_up_to_the_next_stop() {
        let lines = "\tfoo(x)\n\tbar\nbaz";
        let aligned = escape_lines(lines, place(Align::Right, 24));
        let columns_of = |l: &str| columns(l.trim_end_matches('\\').trim_end(), 8);
        assert_eq!(columns_of(aligned.lines().next().unwrap()), 14);
        assert!(
            aligned.lines().take(2).all(|l| columns(l, 8) == 24),
            "{aligned}"
        );
    }
}
