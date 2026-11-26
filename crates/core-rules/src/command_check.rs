//! `unchecked-command`: a command of the option `commands` that starts a line is followed, on its
//! own line or within the next lines, by none of the texts of the option `checks`. An entry of
//! `commands` is a word, or `word:N` for a window of N lines instead of the option `window`. The
//! words are compared without case; a command that does not start its line (after `if`, `&`, `(`,
//! `@`, a path) is left alone. Reported on the command word.
//!
//! concept: unchecked-command
//! what: A listed command at the start of a line has no check of its result on its line or the lines right after.
//! tools: blinter W002, W003, W025
//! rules: unchecked-command
//! reads: rule:unchecked-command

use crate::options::{Check, Opt, OptionDecl};
use crate::rule;
use crate::rule::{FileContext, RuleContext};

rule! {
    pub UncheckedCommand {
        id: "unchecked-command",
        category: Suspicious,
        default: Off,
        description: "Reports a command listed in the option `commands` that starts a line and has \
              no text of the option `checks` (compared ignoring case) on its line or the lines \
              after it, as many as the option `window` says (an entry `word:N` of `commands` \
              says N for that command). A command after `if`, `&`, `(` or a path is not \
              reported.",
        why: "A command that can fail (a copy, a delete, a registry or service change) does not \
              stop the script, which then goes on as if it had worked.\n\
              To fix, test the result on the next line, or write why a failure does not matter.",
        example: "copy a.txt b.txt   // reported when no `errorlevel` follows\necho done",
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
            description: "The commands that need a check, `word` or `word:N` (a window of N lines).",
        },
        OptionDecl {
            name: "window",
            default: Opt::Int(3),
            check: Check::Any,
            description: "The lines after the command's line that may hold the check.",
        },
        OptionDecl {
            name: "checks",
            default: Opt::List(&[]),
            check: Check::Any,
            description: "The texts that count as a check of the result (ignoring case).",
        }],
    }

    fn check_file(&self, file: &FileContext<'_>, ctx: &mut RuleContext<'_>) {
        let commands = ctx.options.list("commands");
        let checks: Vec<String> = (ctx.options.list("checks").into_iter())
            .map(str::to_ascii_lowercase)
            .collect();
        if commands.is_empty() || checks.is_empty() {
            return;
        }
        let window = usize::try_from(ctx.options.int("window")).unwrap_or(0);
        let (g, text) = (file.grammar, file.text);
        for t in file.root.descendants_with_tokens().filter_map(|e| e.into_token()) {
            if !ctx.options.list_has("kinds", g.name(t.kind()), false) {
                continue;
            }
            let range = t.text_range();
            let word = &text[range];
            let Some(lines) = commands.iter().find_map(|c| {
                let (name, n) = c.split_once(':').map_or((*c, None), |(a, b)| (a, b.parse().ok()));
                name.eq_ignore_ascii_case(word).then_some(n.unwrap_or(window))
            }) else {
                continue;
            };
            let start = usize::from(range.start());
            let line = text[..start].rfind('\n').map_or(0, |n| n + 1);
            if !text[line..start].trim().is_empty() {
                continue;
            }
            let mut end = start;
            for _ in 0..=lines {
                end = text[end..].find('\n').map_or(text.len(), |n| end + n + 1);
            }
            let seen = text[line..end].to_ascii_lowercase();
            if !checks.iter().any(|c| seen.contains(c.as_str())) {
                ctx.report(range, format!("`{word}` can fail and its result is not checked"));
            }
        }
    }
}
