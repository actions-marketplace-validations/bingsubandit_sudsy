//! `commented-block`: an outermost block of the node kind of the option `block`, whose parent
//! statement (one of the two ancestors above it) has a kind of the option `parents`, holds at least
//! `lines` lines of code and has no comment inside or on the line right above it. With the option
//! `indented`, only a block whose body is indented deeper than its opening line counts. Reported
//! on the opening token of the block.
//!
//! concept: commented-block
//! what: A long block of code has no comment inside it or right above it.
//! tools: blinter S008
//! rules: commented-block
//! reads: rule:commented-block

use crate::options::{Check, Opt, OptionDecl};
use crate::rule;
use crate::rule::{FileContext, RuleContext};
use core_syntax::SyntaxNode;
use std::collections::HashSet;

rule! {
    pub CommentedBlock {
        id: "commented-block",
        category: Style,
        default: Off,
        description: "Reports an outermost block of the kind named in the option `block` that is \
              the body of a statement of the kinds in `parents`, has at least `lines` lines of \
              code, and has no comment inside it or on the line right above it. With `indented`, \
              a block whose body is not indented deeper than its opening line is left out.",
        why: "A long block is a step of the script with its own purpose, and a reader who finds \
              only commands has to work out what it is for.\n\
              To fix, write a comment above the block that says what it does and why.",
        example: "if %a%==1 (   // reported: five lines, no comment\n    one\n    two\n    three\n    four\n)",
        options: [
        OptionDecl {
            name: "block",
            default: Opt::List(&[]),
            check: Check::Any,
            description: "The node kinds of a block.",
        },
        OptionDecl {
            name: "parents",
            default: Opt::List(&[]),
            check: Check::Any,
            description: "The node kinds of the statements whose block is checked.",
        },
        OptionDecl {
            name: "lines",
            default: Opt::Int(5),
            check: Check::Any,
            description: "The least number of lines of code in the block.",
        },
        OptionDecl {
            name: "indented",
            default: Opt::Bool(true),
            check: Check::Any,
            description: "Only blocks whose body is indented deeper than the line that opens them.",
        }],
    }

    fn check_file(&self, file: &FileContext<'_>, ctx: &mut RuleContext<'_>) {
        let (g, text) = (file.grammar, file.text);
        let starts: Vec<usize> = std::iter::once(0)
            .chain(text.match_indices('\n').map(|(i, _)| i + 1))
            .collect();
        let line_of = |offset: usize| starts.partition_point(|&s| s <= offset) - 1;
        let comment_lines: HashSet<usize> = (file.root.descendants_with_tokens())
            .filter_map(|e| e.into_token())
            .filter(|t| g.has_tag(t.kind(), "comment"))
            .map(|t| line_of(usize::from(t.text_range().start())))
            .collect();
        let min = usize::try_from(ctx.options.int("lines")).unwrap_or(0);
        let indented = ctx.options.bool("indented");
        let kind_is = |n: &SyntaxNode, option: &str| ctx.options.list_has(option, g.name(n.kind()), false);
        for node in file.root.descendants().filter(|n| kind_is(n, "block")) {
            if node.ancestors().skip(1).any(|a| kind_is(&a, "block"))
                || !node.ancestors().skip(1).take(2).any(|a| kind_is(&a, "parents"))
            {
                continue;
            }
            let range = node.text_range();
            let (open, close) = (line_of(usize::from(range.start())), line_of(usize::from(range.end()) - 1));
            if (open..=close).any(|l| comment_lines.contains(&l)) || (open > 0 && comment_lines.contains(&(open - 1))) {
                continue;
            }
            let body: Vec<&str> = (open + 1..close)
                .map(|l| &text[starts[l]..starts.get(l + 1).copied().unwrap_or(text.len())])
                .filter(|l| !l.trim().is_empty())
                .collect();
            if body.len() < min {
                continue;
            }
            let indent = |l: &str| l.len() - l.trim_start().len();
            let opener = &text[starts[open]..starts[open + 1]];
            if indented && body.first().is_none_or(|b| indent(b) <= indent(opener)) {
                continue;
            }
            if let Some(t) = node.first_token() {
                ctx.report(t.text_range(), "this block has no comment saying what it does".to_string());
            }
        }
    }
}
