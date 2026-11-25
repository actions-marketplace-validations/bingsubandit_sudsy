//! The doc comment of a language whose doc comments are the line comments right above a
//! declaration (Go `// Name does ...`): the run of comments before the first code token of the
//! declaration, with no blank line inside it or between it and the declaration. Params of the
//! families that read doc comments ([`Docs::read`]):
//! - `doc-lines`: the line comment prefixes (`//`) of such a language (a block comment counts too);
//!   without it a `/** */` comment is the doc comment;
//! - `doc-skip`: a regex for the text after the prefix of a comment line that is no text (a
//!   directive: `go:generate x`), the line is dropped;
//! - `group-comment`: kinds of the parent whose own doc comment stands for a declaration that has
//!   none (`const ( ... )`), `group-open`: the token that makes such a parent a group (`(`).
//!
//! The first line of the text is its first line that is not empty; a doc comment whose first line
//! starts with `Deprecated: ` has none.
//!
//! concept: doc-run
//! what: The run of line comments right above a declaration is its doc comment; its first line.
//! tools: revive exported, package-comments
//! rules: missing-doc-comment, doc-first-word
//! reads: family:missing-doc-comment, family:doc-comment

use crate::Params;
use crate::regex::Re;
use crate::tree::is_code;
use core_syntax::{Grammar, SyntaxNode, SyntaxToken, TextSize};

/// A doc comment found: its first comment and the first line of its text.
pub struct Doc {
    pub first: SyntaxToken,
    pub line: String,
}

pub struct Docs {
    prefixes: Vec<String>,
    skip: Option<Re>,
    group: Vec<String>,
    open: Option<String>,
}

impl Docs {
    pub fn read(p: &mut Params) -> Result<Docs, String> {
        let words = |t: Option<String>| -> Vec<String> {
            t.unwrap_or_default()
                .split_whitespace()
                .map(str::to_string)
                .collect()
        };
        let skip = match p.optional("doc-skip") {
            Some(s) => Some(Re::new(&s)?),
            None => None,
        };
        Ok(Docs {
            prefixes: words(p.optional("doc-lines")),
            skip,
            group: words(p.optional("group-comment")),
            open: p.optional("group-open"),
        })
    }

    /// Does the language write its doc comments as runs of line comments?
    pub fn lines(&self) -> bool {
        !self.prefixes.is_empty()
    }

    /// The doc comment of the declaration in `holder`: its own, else its group's.
    pub fn of(&self, g: &Grammar, holder: &SyntaxNode) -> Option<Doc> {
        let own = self.own(g, holder).filter(|d| !d.line.is_empty());
        let group = || {
            let (group, _) = self.group_of(g, holder)?;
            self.own(g, &group).filter(|d| !d.line.is_empty())
        };
        own.or_else(group)
    }

    /// Is there a run of comments right above the first code token of `holder`?
    pub fn exists(&self, g: &Grammar, holder: &SyntaxNode) -> bool {
        !self.run(g, holder).is_empty()
    }

    /// Does the group of the declaration in `holder` have a doc comment of its own?
    pub fn group_has_doc(&self, g: &Grammar, holder: &SyntaxNode) -> bool {
        self.group_of(g, holder)
            .and_then(|(group, _)| self.own(g, &group))
            .is_some_and(|d| !d.line.is_empty())
    }

    /// Does the group of the declaration in `holder` have the token `group-open`?
    pub fn in_open_group(&self, g: &Grammar, holder: &SyntaxNode) -> bool {
        self.group_of(g, holder).is_some_and(|(_, open)| open)
    }

    /// The group of the declaration in `holder`: the nearest of its two enclosing nodes whose kind
    /// is in `group-comment`, and whether a token of `group-open` stands in the nodes up to it.
    fn group_of(&self, g: &Grammar, holder: &SyntaxNode) -> Option<(SyntaxNode, bool)> {
        let mut open = false;
        for a in holder.ancestors().skip(1).take(2) {
            open |= self.open.as_ref().is_some_and(|o| {
                a.children_with_tokens()
                    .filter_map(|e| e.into_token())
                    .any(|t| t.text() == o)
            });
            if self.group.iter().any(|k| k == g.name(a.kind())) {
                return Some((a, open));
            }
        }
        None
    }

    fn own(&self, g: &Grammar, holder: &SyntaxNode) -> Option<Doc> {
        let run = self.run(g, holder);
        let first = run.first()?.clone();
        Some(Doc {
            line: self.first_line(g, &run),
            first,
        })
    }

    /// The comments right above the first code token of `holder`, in order.
    fn run(&self, g: &Grammar, holder: &SyntaxNode) -> Vec<SyntaxToken> {
        match self.last_run(g, holder) {
            (run, true) => run,
            _ => Vec::new(),
        }
    }

    /// The comment run of `holder` that is written right above its first code token but ends
    /// before a blank line (a detached doc comment) when its first line starts with `prefix`:
    /// the end of its last comment.
    pub fn detached(&self, g: &Grammar, holder: &SyntaxNode, prefix: &str) -> Option<TextSize> {
        let (run, adjacent) = self.last_run(g, holder);
        if adjacent || !self.first_line(g, &run).starts_with(prefix) {
            return None;
        }
        run.last().map(|t| t.text_range().end())
    }

    /// The last run of comments before the first code token of `holder`, and whether no blank line
    /// separates it from that token.
    fn last_run(&self, g: &Grammar, holder: &SyntaxNode) -> (Vec<SyntaxToken>, bool) {
        let mut run: Vec<SyntaxToken> = Vec::new();
        let mut newlines = 0;
        let mut seen_newline = false;
        let after_code = ends_line_of_code(g, holder);
        for t in holder
            .descendants_with_tokens()
            .filter_map(|e| e.into_token())
        {
            if is_code(&t, g) {
                break;
            }
            let text = t.text();
            if !g.has_tag(t.kind(), "comment") {
                let n = text.matches('\n').count();
                newlines += n;
                seen_newline |= n > 0;
                continue;
            }
            if !seen_newline && after_code {
                // The comment ends the line of the previous code: it is no doc comment.
                continue;
            }
            if newlines >= 2 {
                run.clear();
            }
            if self.prefixes.iter().any(|p| text.starts_with(p.as_str()))
                || is_block(g, text).is_some()
            {
                run.push(t.clone());
            } else {
                run.clear();
            }
            newlines = 0;
            seen_newline |= text.contains('\n');
        }
        (run, newlines < 2)
    }

    /// The first line of the text of the comments: a line comment is its line, a block comment
    /// its lines; directive lines and empty lines do not count.
    fn first_line(&self, g: &Grammar, run: &[SyntaxToken]) -> String {
        for t in run {
            let text = t.text();
            let lines: Vec<&str> = if let Some(inner) = is_block(g, text) {
                inner.lines().collect()
            } else {
                let Some(prefix) = self.prefixes.iter().find(|p| text.starts_with(p.as_str()))
                else {
                    continue;
                };
                let body = &text[prefix.len()..];
                if self.skip.as_ref().is_some_and(|re| re.is_match(body)) {
                    continue;
                }
                vec![body]
            };
            if let Some(line) = lines.into_iter().map(str::trim).find(|l| !l.is_empty()) {
                return if line.starts_with("Deprecated: ") {
                    String::new()
                } else {
                    line.to_string()
                };
            }
        }
        String::new()
    }
}

/// The inside of a block comment.
fn is_block<'a>(g: &Grammar, text: &'a str) -> Option<&'a str> {
    let (open, close) = g.comments.block.as_ref()?;
    text.strip_prefix(open.as_str())?
        .strip_suffix(close.as_str())
}

/// Does `holder` start on the line of code before it (so a comment there ends that line)?
fn ends_line_of_code(g: &Grammar, holder: &SyntaxNode) -> bool {
    let mut at = holder.first_token().and_then(|t| t.prev_token());
    while let Some(t) = at {
        if is_code(&t, g) {
            return true;
        }
        if t.text().contains('\n') {
            return false;
        }
        at = t.prev_token();
    }
    false
}
