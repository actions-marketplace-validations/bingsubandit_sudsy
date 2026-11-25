//! `statement-indent`: the statements of a flat command list sit at the column their block depth
//! gives, `depth * size` ([`core_syntax::block_pair`]; the language's block words come from
//! [`crate::RuleHooks::blocks`]). The statements are the children of the root node, a statement's
//! name is its first significant token. Checked, when the token is the first on its line:
//! - a statement: its column is its depth times `size`;
//! - the closing `)` of a statement that spans lines: the column of the statement's depth;
//! - a comment between statements: its column is the depth after the statement before it or the
//!   depth of the statement after it (a comment before `endif` may sit in the body or at the
//!   `endif`).
//!
//! With `allow_unindented` a token at column 0 is accepted at any depth.
//!
//! The fix rewrites the blanks in front of the token. A language without block words is not
//! checked.
//!
//! concept: statement-indent
//! what: Statements, closing parentheses and comments of a flat command list at the column their block depth gives.
//! tools: cmake-lint C0307
//! rules: statement-indent
//! reads: path:block_pair::Block

use crate::diagnostic::Edit;
use crate::options::{Check, Opt, OptionDecl};
use crate::rule;
use crate::rule::{FileContext, RuleContext};
use core_syntax::block_pair::Pairing;
use core_syntax::{SyntaxElement, SyntaxToken, TextRange, TextSize};

rule! {
    pub StatementIndent {
        id: "statement-indent",
        category: Style,
        default: Off,
        fix: Safe,
        description: "A statement, its closing parenthesis or a comment between statements is \
                  not at the column its block depth gives (depth times `size`). Only for \
                  languages whose commands open and close blocks.",
        example: "if(A)\nset(x)   // reported: depth 1 wants column 2\nendif()",
        options: [
            OptionDecl {
                name: "size",
                default: Opt::Int(2),
                check: Check::Any,
                description: "Columns per block depth.",
            },
            OptionDecl {
                name: "allow_unindented",
                default: Opt::Bool(false),
                check: Check::Any,
                description: "Accept a token at column 0 whatever its depth.",
            },
        ],
    }

    fn check_file(&self, file: &FileContext<'_>, ctx: &mut RuleContext<'_>) {
        let blocks = file.hooks.blocks();
        if blocks.is_empty() {
            return;
        }
        let size = usize::try_from(ctx.options.int("size")).unwrap_or(2);
        let free = ctx.options.bool("allow_unindented");
        let g = file.grammar;
        let mut pairing = Pairing::new(blocks);
        let mut after = 0u32;
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
            let mut first: Option<SyntaxToken> = None;
            let mut last: Option<SyntaxToken> = None;
            for t in node.descendants_with_tokens().filter_map(|e| e.into_token()) {
                if g.is_trivia(t.kind()) {
                    if first.is_none() && g.has_tag(t.kind(), "comment") {
                        comments.push(t);
                    }
                    continue;
                }
                first.get_or_insert_with(|| t.clone());
                last = Some(t);
            }
            let (Some(name), Some(last)) = (first, last) else {
                continue;
            };
            let at = usize::from(name.text_range().start());
            let step = pairing.word(name.text(), at as u32);
            for c in comments.drain(..) {
                let Some(col) = column(file.text, &c).filter(|c| *c > 0 || !free) else {
                    continue;
                };
                if col != after as usize * size && col != step.depth as usize * size {
                    wrong(file, ctx, &c, col, step.depth as usize * size);
                }
            }
            let start = column(file.text, &name).filter(|c| *c > 0 || !free);
            if let Some(col) = start.filter(|c| *c != step.column(size)) {
                wrong(file, ctx, &name, col, step.column(size));
            }
            if last.text_range() != name.text_range()
                && let Some(col) = column(file.text, &last)
                    .filter(|c| (*c > 0 || !free) && *c != step.column(size))
            {
                wrong(file, ctx, &last, col, step.column(size));
            }
            after = step.after();
        }
        for c in comments {
            let Some(col) = column(file.text, &c).filter(|c| *c > 0) else {
                continue;
            };
            if col != after as usize * size {
                wrong(file, ctx, &c, col, 0);
            }
        }
    }
}

/// The start of the line holding `at`.
fn line_start(text: &str, at: usize) -> usize {
    text[..at].rfind('\n').map_or(0, |i| i + 1)
}

/// The column of the token when only spaces stand before it on its line (a tab in the
/// indentation is a `no-hard-tabs` finding, its width is not guessed).
fn column(text: &str, t: &SyntaxToken) -> Option<usize> {
    let at = usize::from(t.text_range().start());
    let before = &text[line_start(text, at)..at];
    before
        .chars()
        .all(|c| c == ' ')
        .then(|| before.chars().count())
}

fn wrong(
    file: &FileContext<'_>,
    ctx: &mut RuleContext<'_>,
    t: &SyntaxToken,
    col: usize,
    expected: usize,
) {
    let at = usize::from(t.text_range().start());
    let from = line_start(file.text, at);
    let blanks = TextRange::new(TextSize::from(from as u32), TextSize::from(at as u32));
    ctx.report_fix(
        t.text_range(),
        format!("indentation is {col} columns; expected {expected}"),
        Some((
            "Fix the indentation",
            vec![Edit::replace(blanks, " ".repeat(expected))],
        )),
    );
}
