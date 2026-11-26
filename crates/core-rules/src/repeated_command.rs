//! `repeated-command`: a command word of the option `commands` (a token of a kind of the option
//! `kinds`, compared without case) is met `nth` times or more in a file. An entry `word arg` of
//! `commands` counts only the uses with `arg` after the word in its command; `word arg:N` reports from
//! the Nth use instead of `nth`. Each entry is counted on its own. Reported on the Nth one and every
//! later one; with the option `first-exempt`, a use in the first command of the file is not reported.
//!
//! concept: repeated-command
//! what: A command that should be used once per file, or once per scope, is written a second time.
//! tools: blinter P020, P024
//! rules: repeated-command
//! reads: rule:repeated-command

use crate::options::{Check, Opt, OptionDecl};
use crate::rule;
use crate::rule::{FileContext, RuleContext};
use core_syntax::SyntaxToken;

/// Whether a word of `words` (ignoring case) follows `t` in its command; true when `words` is empty.
pub(crate) fn followed_by(t: &SyntaxToken, words: &[&str]) -> bool {
    words.is_empty()
        || t.parent().is_some_and(|p| {
            p.descendants_with_tokens()
                .filter_map(|e| e.into_token())
                .any(|a| {
                    a.text_range().start() >= t.text_range().end()
                        && words
                            .iter()
                            .any(|w| w.eq_ignore_ascii_case(a.text().trim()))
                })
        })
}

rule! {
    pub RepeatedCommand {
        id: "repeated-command",
        category: Style,
        default: Off,
        description: "Reports the `nth` use (the option, default 2) and every later one of a command \
              listed in the option `commands`; an entry `word arg` counts only the uses with `arg` after \
              the word; the command words are the tokens of the kinds in `kinds`.",
        why: "A second use of a command that scopes or prepares the whole file adds cost and hides \
              which one holds.\n\
              To fix, keep one use, or close the first scope before the next.",
        example: "setlocal\nsetlocal   // reported: the second use",
        options: [
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
            description: "The commands that are counted (ignoring case): `word`, or `word`, `word arg`, with `:N` at the end for its own `nth`.",
        },
        OptionDecl {
            name: "nth",
            default: Opt::Int(2),
            check: Check::Any,
            description: "The use that is reported.",
        },
        OptionDecl {
            name: "first-exempt",
            default: Opt::Bool(false),
            check: Check::Any,
            description: "A use in the first command of the file is counted but not reported.",
        }],
    }

    fn check_file(&self, file: &FileContext<'_>, ctx: &mut RuleContext<'_>) {
        let nth = usize::try_from(ctx.options.int("nth")).unwrap_or(2).max(1);
        let entries: Vec<(&str, Vec<&str>, usize)> = ctx
            .options
            .list("commands")
            .into_iter()
            .filter_map(|e| {
                let (e, n) = match e.rsplit_once(':') {
                    Some((head, n)) if n.parse::<usize>().is_ok() => (head, n.parse().ok()?),
                    _ => (e, nth),
                };
                let mut w = e.split_whitespace();
                Some((w.next()?, w.collect(), n))
            })
            .collect();
        let mut seen = vec![0; entries.len()];
        let first = file.root.children().next().map(|c| c.text_range());
        for t in file.root.descendants_with_tokens().filter_map(|e| e.into_token()) {
            if !ctx.options.list_has("kinds", file.grammar.name(t.kind()), false) {
                continue;
            }
            for (i, (word, args, from)) in entries.iter().enumerate() {
                if !word.eq_ignore_ascii_case(&file.text[t.text_range()]) || !followed_by(&t, args) {
                    continue;
                }
                seen[i] += 1;
                let in_first = ctx.options.bool("first-exempt") && first.is_some_and(|f| f.contains_range(t.text_range()));
                if seen[i] >= *from && !in_first {
                    ctx.report(t.text_range(), format!("`{}` is used here for the {}th time", t.text().trim(), seen[i]));
                }
            }
        }
    }
}
