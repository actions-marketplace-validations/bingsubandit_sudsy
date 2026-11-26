//! `documented-subroutine`: a token of a kind of the option `labels` (`:name`) that a command of the
//! option `commands` (`call`, the word of a kind of the option `kinds`) names as its first argument
//! has no comment on the line above, two lines above with a blank line between, or on the line
//! below. Reported on the label.
//!
//! concept: documented-subroutine
//! what: A label that is called like a function has no comment next to it.
//! tools: blinter S018
//! rules: documented-subroutine
//! reads: rule:documented-subroutine

use crate::argument_prefix::argument_after;
use crate::options::{Check, Opt, OptionDecl};
use crate::rule;
use crate::rule::{FileContext, RuleContext};
use std::collections::HashSet;

rule! {
    pub DocumentedSubroutine {
        id: "documented-subroutine",
        category: Style,
        default: Off,
        description: "Reports a label of a kind in the option `labels` that a command in `commands` \
              calls (the first argument of the command is the label) and that has no comment on \
              the line above, two lines above with a blank line between, or on the line below.",
        why: "A label that is called is a function of the script, and its name is all a reader \
              sees at the call.\n\
              To fix, write a comment next to the label that says what it takes and does.",
        example: "call :clean\n:clean   // reported: no comment\nexit /b",
        options: [
        OptionDecl {
            name: "labels",
            default: Opt::List(&[]),
            check: Check::Any,
            description: "The token kinds of a label.",
        },
        OptionDecl {
            name: "kinds",
            default: Opt::List(&[]),
            check: Check::Any,
            description: "The token kinds of a command word.",
        },
        OptionDecl {
            name: "commands",
            default: Opt::List(&[]),
            check: Check::Any,
            description: "The commands whose first argument names a label (ignoring case).",
        }],
    }

    fn check_file(&self, file: &FileContext<'_>, ctx: &mut RuleContext<'_>) {
        let (g, text) = (file.grammar, file.text);
        let starts: Vec<usize> = std::iter::once(0)
            .chain(text.match_indices('\n').map(|(i, _)| i + 1))
            .collect();
        let line_of = |offset: usize| starts.partition_point(|&s| s <= offset) - 1;
        let blank = |l: usize| {
            let end = starts.get(l + 1).copied().unwrap_or(text.len());
            text[starts[l]..end].trim().is_empty()
        };
        let mut comments: HashSet<usize> = HashSet::new();
        let mut called: HashSet<String> = HashSet::new();
        let mut labels = Vec::new();
        for t in file.root.descendants_with_tokens().filter_map(|e| e.into_token()) {
            let name = g.name(t.kind());
            if g.has_tag(t.kind(), "comment") {
                comments.insert(line_of(usize::from(t.text_range().start())));
            } else if ctx.options.list_has("labels", name, false) {
                labels.push(t);
            } else if ctx.options.list_has("kinds", name, false)
                && ctx.options.list_has("commands", t.text(), true)
                && let Some(arg) = argument_after(&t)
            {
                called.insert(text[arg.text_range()].trim().to_ascii_lowercase());
            }
        }
        for t in labels {
            let name = t.text().trim().to_ascii_lowercase();
            if !called.contains(&name) {
                continue;
            }
            let l = line_of(usize::from(t.text_range().start()));
            let documented = (l > 0 && comments.contains(&(l - 1)))
                || (l > 1 && blank(l - 1) && comments.contains(&(l - 2)))
                || comments.contains(&(l + 1));
            if !documented {
                ctx.report(t.text_range(), format!("`{}` is called but has no comment next to it", t.text().trim()));
            }
        }
    }
}
