//! `documented-command`: a statement whose command is one of the option `commands` (`function`,
//! `macro`) has a comment on the line right before it, and the comment has text. A blank line or
//! other code before it: no documentation; a comment of no text (`#` alone): an empty one.
//!
//! concept: documented-command
//! what: A command of a listed kind has a comment with text on the line right before it.
//! tools: cmake-lint C0111, C0112
//! rules: documented-command

use crate::options::{Check, Opt, OptionDecl};
use crate::rule;
use crate::rule::{FileContext, RuleContext};
use core_syntax::{SyntaxElement, SyntaxToken};

rule! {
    pub DocumentedCommand {
        id: "documented-command",
        category: Style,
        default: Off,
        description: "A command of the `commands` option (a function or macro definition) with no \
                  comment on the line right before it, or with a comment that has no text.",
        example: "function(f)   // reported: no comment before it\nendfunction()",
        options: [
        OptionDecl {
            name: "commands",
            default: Opt::List(&[]),
            check: Check::Any,
            description: "The commands that need a comment (case ignored).",
        },
        OptionDecl {
            name: "doc_kinds",
            default: Opt::List(&[]),
            check: Check::Any,
            description: "The kinds of comment token that document (none listed: every comment does); a comment of another kind counts as one with no text.",
        }],
    }

    fn check_file(&self, file: &FileContext<'_>, ctx: &mut RuleContext<'_>) {
        let commands = ctx.options.list("commands");
        if commands.is_empty() {
            return;
        }
        let kinds = ctx.options.list("doc_kinds");
        let g = file.grammar;
        // the comments since the previous statement
        let mut comments: Vec<SyntaxToken> = Vec::new();
        for child in file.root.children_with_tokens() {
            let node = match child {
                SyntaxElement::Token(t) => {
                    if g.has_tag(t.kind(), "comment") {
                        comments.push(t);
                    }
                    continue;
                }
                SyntaxElement::Node(n) => n,
            };
            let mut first = None;
            for t in node.descendants_with_tokens().filter_map(|e| e.into_token()) {
                if !g.is_trivia(t.kind()) {
                    first = Some(t);
                    break;
                }
                if g.has_tag(t.kind(), "comment") {
                    comments.push(t);
                }
            }
            let Some(name) = first else {
                continue;
            };
            let before = std::mem::take(&mut comments);
            if !commands.iter().any(|c| c.eq_ignore_ascii_case(name.text())) {
                continue;
            }
            let start = usize::from(name.text_range().start());
            let line = file.text[..start].rfind('\n').map_or(0, |i| i + 1);
            // the run of comments on the lines right above, as one documentation
            let mut above = Vec::new();
            let mut next = line;
            for c in before.iter().rev() {
                let (start, end) = (
                    usize::from(c.text_range().start()),
                    usize::from(c.text_range().end()),
                );
                let first = file.text[..start].rfind('\n').map_or(0, |i| i + 1);
                // a line break only between this comment and the next line, and no code before it
                let gap = &file.text[end.min(next)..next];
                let alone = file.text[first..start].trim().is_empty();
                if next == 0 || !alone || !matches!(gap, "\n" | "\r\n") {
                    break;
                }
                above.push(c);
                next = first;
            }
            let has_text = |c: &&SyntaxToken| {
                !g.comment_body(c.text()).trim().is_empty()
                    && (kinds.is_empty() || kinds.contains(&g.name(c.kind())))
            };
            let message = if above.is_empty() {
                "no comment before the command"
            } else if above.iter().any(has_text) {
                continue;
            } else {
                "the comment before the command has no text"
            };
            ctx.report(name.text_range(), message);
        }
    }
}
