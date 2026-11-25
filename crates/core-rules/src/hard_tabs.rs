//! `no-hard-tabs`: a tab character anywhere in a line, one report per run of tabs. Tokens of the
//! kinds the option `ignore_kinds` names (front matter, code) are not looked at.
//!
//! concept: hard-tabs
//! what: Tab characters in the text of a file, except inside tokens of named kinds.
//! tools: markdownlint MD010

use crate::options::{Check, Opt, OptionDecl};
use crate::rule;
use crate::rule::{FileContext, RuleContext};
use core_syntax::{TextRange, TextSize};

rule! {
    pub NoHardTabs {
        id: "no-hard-tabs",
        category: Style,
        default: Off,
        description: "Reports a tab character in a line; a run of tabs is one report. The option \
              `ignore_kinds` lists token kinds whose tabs are not reported (for example the code \
              of a Markdown fence), and `indentation_only` reports only the tabs in the white \
              space that starts a line.",
        why: "A tab is as wide as each reader's editor and tool makes it, so text that is \
              aligned with tabs looks different in a diff, in a review and on the web. Spaces \
              look the same everywhere. Some formats (YAML indentation) do not accept tabs at \
              all.\n\
              To fix, replace the tab with spaces.",
        example: "\tindented with a tab   // reported at the tab",
        options: [
            OptionDecl {
                name: "ignore_kinds",
                default: Opt::List(&[]),
                check: Check::Any,
                description: "Kinds of tokens whose tabs are not reported (for example the code of a fence).",
            },
            OptionDecl {
                name: "indentation_only",
                default: Opt::Bool(false),
                check: Check::Any,
                description: "Report only the tabs in the whitespace that starts a line.",
            },
        ],
    }

    fn check_file(&self, file: &FileContext<'_>, ctx: &mut RuleContext<'_>) {
        if !file.text.contains('\t') {
            return;
        }
        let skipped = crate::tree::token_ranges(
            file.root,
            file.grammar,
            ctx.options.list("ignore_kinds").as_slice(),
        );
        let bytes = file.text.as_bytes();
        let leading = ctx.options.bool("indentation_only");
        // With `indentation_only`: whether only blanks lie between the line start and `at`.
        let mut line_start = true;
        let mut at = 0;
        while at < bytes.len() {
            let b = bytes[at];
            if b == b'\n' || b == b'\r' {
                line_start = true;
            } else if b != b'\t' && b != b' ' {
                line_start = false;
            }
            if bytes[at] != b'\t' || (leading && !line_start) {
                at += 1;
                continue;
            }
            let start = at;
            while bytes.get(at) == Some(&b'\t') {
                at += 1;
            }
            let range = TextRange::new(TextSize::from(start as u32), TextSize::from(at as u32));
            if !skipped.iter().any(|s| s.contains_range(range)) {
                ctx.report(range, "hard tab");
            }
        }
    }
}
