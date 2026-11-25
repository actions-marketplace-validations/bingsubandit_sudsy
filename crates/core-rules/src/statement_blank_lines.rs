//! `statement-blank-lines`: no more than `max` blank lines (empty or blanks only) stand right
//! before a statement (a child node of the root that starts its line). The file start takes one
//! more: no line before it ends. The arms and closers of the language's blocks
//! ([`crate::RuleHooks::blocks`]) are left out. Reported on the statement's first token.
//!
//! concept: statement-blank-lines
//! what: A run of blank lines before a statement is no longer than the option allows.
//! tools: cmake-lint C0305
//! rules: statement-blank-lines
//! reads: path:block_pair::Block

use crate::options::{Check, Opt, OptionDecl};
use crate::rule;
use crate::rule::{FileContext, RuleContext};
use core_syntax::block_pair::{Pairing, Role};

rule! {
    pub StatementBlankLines {
        id: "statement-blank-lines",
        category: Style,
        default: Off,
        description: "Reports more than `max` blank lines (default 1) right before a top-level statement \
              (a direct child of the file that starts its line). The first statement of a file \
              may have one more. Block arms and closers of the language (`else`, `endif`) are \
              not checked. A line with only white space counts as blank. The finding is on the \
              first token of the statement.",
        why: "A run of blank lines is a visual gap that separates parts of a file. One blank \
              line separates statements well, and more leave holes that make the file look as if \
              something is missing.\n\
              To fix, delete the extra blank lines.",
        example: "set(a 1)\n\n\nset(b 2)   // reported: 2 blank lines",
        options: [OptionDecl {
            name: "max",
            default: Opt::Int(1),
            check: Check::Any,
            description: "The longest run of blank lines allowed before a statement.",
        }],
    }

    fn check_file(&self, file: &FileContext<'_>, ctx: &mut RuleContext<'_>) {
        let max = usize::try_from(ctx.options.int("max")).unwrap_or(1);
        let g = file.grammar;
        let mut pairing = Pairing::new(file.hooks.blocks());
        for node in file.root.children() {
            let Some(first) = node
                .descendants_with_tokens()
                .filter_map(|e| e.into_token())
                .find(|t| !g.is_trivia(t.kind()))
            else {
                continue;
            };
            let at = usize::from(first.text_range().start());
            if matches!(pairing.word(first.text(), at as u32).role, Role::Arm | Role::Close) {
                continue;
            }
            let before = &file.text[..at];
            let line = before.rfind('\n').map_or(0, |i| i + 1);
            if !before[line..].trim().is_empty() {
                continue;
            }
            let lines: Vec<&str> = file.text[..line].split('\n').collect();
            // the last item is the empty text after the final line break
            let blank = lines.iter().rev().skip(1).take_while(|l| l.trim().is_empty()).count();
            let extra = usize::from(blank + 1 == lines.len());
            if blank > max + extra {
                ctx.report(
                    first.text_range(),
                    format!("{blank} blank lines before the statement; at most {max}"),
                );
            }
        }
    }
}
