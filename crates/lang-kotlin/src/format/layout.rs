//! The whole file as text: the line breaks and blanks between the tokens are planned, the
//! indentation of each line follows from the planned lines, and the tokens are printed with
//! them. Tokens are never changed; comments move only with their lines.

use super::indent::{Indent, closes};
use super::plan::{Item, Plan};
use super::src::Source;
use core_format::{Doc, FormatCtx, doc};
use core_syntax::SyntaxNode;

/// The output so far and the indentation of each of its lines.
struct Out {
    text: String,
    line_ws: Vec<String>,
}

impl Out {
    fn at_line_start(&self) -> bool {
        self.text.is_empty() || self.text.ends_with('\n')
    }

    fn trim_end(&mut self) {
        let len = self.text.trim_end_matches([' ', '\t']).len();
        self.text.truncate(len);
    }

    /// Writes `s` (which may span lines) and records the indentation of the lines it starts.
    fn push(&mut self, s: &str) {
        let mut lines = s.split('\n');
        self.text.push_str(lines.next().unwrap_or(""));
        for line in lines {
            self.text.push('\n');
            self.text.push_str(line);
            self.line_ws.push(leading_ws(line));
        }
    }

    /// Starts a line with `ws`.
    fn indent(&mut self, ws: &str) {
        self.text.push_str(ws);
        *self.line_ws.last_mut().expect("a line") = ws.to_string();
    }

    fn column(&self) -> usize {
        self.text.len() - self.text.rfind('\n').map_or(0, |p| p + 1)
    }
}

fn leading_ws(line: &str) -> String {
    line[..line.len() - line.trim_start_matches([' ', '\t']).len()].to_string()
}

pub(super) fn file(root: &SyntaxNode, cx: &FormatCtx<'_>) -> Doc {
    let src = Source::new(root);
    let plan = Plan::new(&src, cx);
    let line_of = plan.lines(&src);
    let indent = Indent {
        src: &src,
        line_of: &line_of,
    };
    let unit = if cx.options.get("indent_style")
        == Some(&core_config::OptionValue::String("tab".into()))
    {
        "\t".to_string()
    } else {
        " ".repeat(cx.options.int("indent_size") as usize)
    };
    let n = src.toks.len();
    let mut out = Out {
        text: String::new(),
        line_ws: vec![String::new()],
    };
    for i in 0..=n {
        let items = &plan.items[i];
        for item in items {
            match item {
                Item::Break(c) => {
                    out.trim_end();
                    for _ in 0..*c {
                        out.text.push('\n');
                        out.line_ws.push(String::new());
                    }
                }
                Item::Comment(text, kept) => {
                    if out.at_line_start() {
                        // Only KDoc and indented line comments are moved to the indent of the code.
                        match kept {
                            Some(w)
                                if !text.starts_with("/**")
                                    && (text.starts_with("/*") || w.is_empty()) =>
                            {
                                out.indent(w);
                            }
                            _ => {
                                let ws = if i == n {
                                    String::new()
                                } else {
                                    let ws = indent.indent_of(
                                        i,
                                        &out.line_ws,
                                        &unit,
                                        &src.gaps[i].indent,
                                    );
                                    if closes(src.toks[i].kind()) {
                                        ws + &unit
                                    } else {
                                        ws
                                    }
                                };
                                out.indent(&ws);
                            }
                        }
                    } else if !out.text.is_empty() {
                        out.text.push(' ');
                    }
                    let col = out.column();
                    push_comment(&mut out, text, col);
                }
                Item::Raw(text) => out.push(&text.replace("\r\n", "\n")),
            }
        }
        if i == n {
            break;
        }
        if src.gap_in_string(i) {
            // Inside a string nothing moves.
        } else if out.at_line_start() {
            let ws = indent.indent_of(i, &out.line_ws, &unit, &src.gaps[i].indent);
            out.indent(&ws);
        } else if plan.space_before[i] {
            out.text.push(' ');
        }
        out.push(&src.toks[i].text().replace("\r\n", "\n"));
    }
    out.trim_end();
    let mut text = out.text;
    if !text.is_empty() {
        text.push('\n');
    }
    doc::concat(vec![doc::text(text), doc::break_parent()])
}

/// A comment; the lines of a block comment that start with `*` are aligned under its `/*`.
fn push_comment(out: &mut Out, text: &str, col: usize) {
    let pad = " ".repeat(col);
    let mut lines = text.split('\n');
    out.text.push_str(lines.next().unwrap_or(""));
    for line in lines {
        let trimmed = line.trim_start_matches([' ', '\t']);
        out.text.push('\n');
        let written = if trimmed.starts_with('*') && (col > 0 || text.starts_with("/**")) {
            format!("{pad} {}", trimmed.trim_end())
        } else {
            line.trim_end().to_string()
        };
        out.text.push_str(&written);
        out.line_ws.push(leading_ws(&written));
    }
}
