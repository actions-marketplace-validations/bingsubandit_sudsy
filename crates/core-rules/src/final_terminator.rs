//! `missing-terminator`: none of the last `lines` lines that hold code (comments left out) holds one
//! of the token sequences of the option `terminators` (compared without case, tokens joined by a
//! blank, so `goto :eof` is two tokens). Reported on the first token of the last line that holds
//! code other than the tokens of the option `skip` (an entry ending in `*` is a prefix).
//!
//! concept: missing-terminator
//! what: A script ends without an explicit exit among its last lines of code.
//! tools: blinter W001
//! rules: missing-terminator
//! reads: rule:missing-terminator

use crate::options::{Check, Opt, OptionDecl};
use crate::rule;
use crate::rule::{FileContext, RuleContext};
use core_syntax::TextRange;

rule! {
    pub MissingTerminator {
        id: "missing-terminator",
        category: Suspicious,
        default: Off,
        description: "Reports a file whose last lines of code (as many as the option `lines`) \
              hold none of the token sequences of the option `terminators`. Reported on the \
              last line of code that is not made of tokens of the option `skip` only.",
        why: "A script that ends without an exit hands its caller whatever the last command left, \
              so a failure in the middle is lost.\n\
              To fix, end the script with an explicit exit that carries the result.",
        example: "call build.cmd\necho done   // reported: no exit at the end",
        options: [
        OptionDecl {
            name: "terminators",
            default: Opt::List(&[]),
            check: Check::Any,
            description: "Token sequences that end a script, such as `exit` or `goto :eof` (ignoring case).",
        },
        OptionDecl {
            name: "skip",
            default: Opt::List(&[]),
            check: Check::Any,
            description: "Tokens that do not count as code when the report line is chosen; an entry ending in `*` is a prefix.",
        },
        OptionDecl {
            name: "lines",
            default: Opt::Int(5),
            check: Check::Any,
            description: "How many of the last lines of code are searched.",
        }],
    }

    fn check_file(&self, file: &FileContext<'_>, ctx: &mut RuleContext<'_>) {
        let (g, text) = (file.grammar, file.text);
        // The lines of code: the range of the first token and the lowercased token texts.
        let mut lines: Vec<(TextRange, usize, Vec<String>)> = Vec::new();
        for t in file.root.descendants_with_tokens().filter_map(|e| e.into_token()) {
            if g.has_tag(t.kind(), "trivia") || g.has_tag(t.kind(), "comment") || t.text().is_empty() {
                continue;
            }
            let range = t.text_range();
            let line = text[..usize::from(range.start())].rfind('\n').map_or(0, |n| n + 1);
            let word = t.text().trim().to_ascii_lowercase();
            match lines.last_mut() {
                Some((_, l, words)) if *l == line => words.push(word),
                _ => lines.push((range, line, vec![word])),
            }
        }
        let terminators: Vec<String> = (ctx.options.list("terminators").into_iter())
            .map(|t| format!(" {} ", t.to_ascii_lowercase()))
            .collect();
        let window = usize::try_from(ctx.options.int("lines")).unwrap_or(0);
        let tail_from = lines.len().saturating_sub(window);
        let ends = lines[tail_from..].iter().any(|(_, _, words)| {
            let joined = format!(" {} ", words.join(" "));
            terminators.iter().any(|t| joined.contains(t.as_str()))
        });
        if ends {
            return;
        }
        let skipped = |w: &String| {
            ctx.options.list("skip").iter().any(|s| match s.strip_suffix('*') {
                Some(prefix) => w.starts_with(&prefix.to_ascii_lowercase()),
                None => s.eq_ignore_ascii_case(w),
            })
        };
        if let Some((range, _, _)) = lines.iter().rev().find(|(_, _, words)| !words.iter().all(skipped)) {
            ctx.report(*range, "the script ends without an explicit exit".to_string());
        }
    }
}
