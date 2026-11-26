//! `unused-opener`: a command word of the option `open` (a token of a kind of `kinds`, compared
//! without case) that has one of the words of the option `args` after it in its command (any, when
//! `args` is empty), while no token of a kind of `use-kinds` in the file matches the regex of the
//! option `use`. Reported on the command word.
//!
//! concept: unused-opener
//! what: A command that prepares a feature or a scope is written in a file that never uses it.
//! tools: blinter P003, P004
//! rules: unused-opener
//! reads: rule:unused-opener

use crate::options::{Check, Opt, OptionDecl};
use crate::repeated_command::followed_by;
use crate::rule;
use crate::rule::{FileContext, RuleContext};
use crate::sorting::cached_regex;

rule! {
    pub UnusedOpener {
        id: "unused-opener",
        category: Suspicious,
        default: Off,
        description: "Reports a command of the option `open`, with one of the words of `args` \
              after it when that option is set, in a file where no token of the kinds in \
              `use-kinds` matches the regex of the option `use`.",
        why: "A command that turns on a feature or opens a scope has a cost, and when nothing in \
              the file uses it, it only misleads a reader.\n\
              To fix, remove the command, or add the use it is meant for.",
        example: "setlocal enabledelayedexpansion\necho done   // reported: no `!name!` is read",
        options: [
        OptionDecl {
            name: "kinds",
            default: Opt::List(&[]),
            check: Check::Any,
            description: "The token kinds of a command word.",
        },
        OptionDecl {
            name: "open",
            default: Opt::List(&[]),
            check: Check::Any,
            description: "The commands that open the feature or scope (ignoring case).",
        },
        OptionDecl {
            name: "args",
            default: Opt::List(&[]),
            check: Check::Any,
            description: "Words one of which must follow the command to make it count (ignoring case); empty: any.",
        },
        OptionDecl {
            name: "use-kinds",
            default: Opt::List(&[]),
            check: Check::Any,
            description: "The token kinds that can use the feature.",
        },
        OptionDecl {
            name: "use",
            default: Opt::Str(""),
            check: Check::Regex,
            description: "A regex the text of a token that uses the feature matches.",
        }],
    }

    fn check_file(&self, file: &FileContext<'_>, ctx: &mut RuleContext<'_>) {
        let Some(uses) = cached_regex(ctx.options.string("use")).filter(|_| !ctx.options.string("use").is_empty())
        else {
            return;
        };
        let g = file.grammar;
        let tokens: Vec<_> = file.root.descendants_with_tokens().filter_map(|e| e.into_token()).collect();
        if tokens.iter().any(|t| ctx.options.list_has("use-kinds", g.name(t.kind()), false) && uses.is_match(t.text())) {
            return;
        }
        for t in &tokens {
            if !ctx.options.list_has("kinds", g.name(t.kind()), false)
                || !ctx.options.list_has("open", &file.text[t.text_range()], true)
            {
                continue;
            }
            let has_args = followed_by(t, &ctx.options.list("args"));
            if has_args {
                ctx.report(t.text_range(), format!("`{}` is never used in this file", t.text().trim()));
            }
        }
    }
}
