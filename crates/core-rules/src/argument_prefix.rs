//! `argument-prefix`: the first argument of the commands of the option `commands` is written in one
//! form in a file, with the option `prefix` or without it, and the first such command sets the
//! form. The words are compared without case; the arguments of the option `except` (compared without
//! the prefix and without case) have one form only and are left out. Reported on the argument of each
//! later command in the other form.
//!
//! concept: argument-prefix
//! what: The first argument of a command is written with a prefix in some places and without it in others.
//! tools: blinter S015
//! rules: argument-prefix
//! reads: rule:argument-prefix

use crate::options::{Check, Opt, OptionDecl};
use crate::rule;
use crate::rule::{FileContext, RuleContext};
use core_syntax::{SyntaxElement, SyntaxToken, TextRange, TextSize};

/// The first child node after a command word: its first argument.
pub(crate) fn argument_after(word: &SyntaxToken) -> Option<SyntaxElement> {
    let after = word.text_range().end();
    word.parent()?
        .children_with_tokens()
        .find(|c| c.text_range().start() >= after && matches!(c, SyntaxElement::Node(_)))
}

rule! {
    pub ArgumentPrefix {
        id: "argument-prefix",
        category: Style,
        default: Off,
        description: "Reports the first argument of a command listed in the option `commands` \
              that is written with the option `prefix` when the first such command of the file \
              has none, or the other way round. Arguments listed in `except` are left out.",
        why: "Two spellings of one thing make a reader wonder whether they differ, and a search \
              for one of them misses the other.\n\
              To fix, write every argument in one form.",
        example: "goto :done\ngoto next   // reported: no colon, the first has one",
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
            description: "The commands whose first argument is checked (ignoring case).",
        },
        OptionDecl {
            name: "prefix",
            default: Opt::Str(""),
            check: Check::Any,
            description: "The text an argument may start with.",
        },
        OptionDecl {
            name: "except",
            default: Opt::List(&[]),
            check: Check::Any,
            description: "Arguments, without the prefix, that have one form only (ignoring case).",
        }],
    }

    fn check_file(&self, file: &FileContext<'_>, ctx: &mut RuleContext<'_>) {
        let prefix = ctx.options.string("prefix");
        if prefix.is_empty() {
            return;
        }
        let mut first: Option<bool> = None;
        for t in file.root.descendants_with_tokens().filter_map(|e| e.into_token()) {
            if !ctx.options.list_has("kinds", file.grammar.name(t.kind()), false)
                || !ctx.options.list_has("commands", &file.text[t.text_range()], true)
            {
                continue;
            }
            let Some(arg) = argument_after(&t) else { continue };
            let raw = &file.text[arg.text_range()];
            let text = raw.trim();
            let start = arg.text_range().start() + TextSize::of(&raw[..raw.len() - raw.trim_start().len()]);
            let (has, bare) = text.strip_prefix(prefix).map_or((false, text), |r| (true, r));
            if bare.is_empty()
                || bare.contains(char::is_whitespace)
                || ctx.options.list_has("except", bare, true)
            {
                continue;
            }
            match first {
                None => first = Some(has),
                Some(f) if f != has => {
                    let message = format!("`{text}` is written in another form than the first one of the file");
                    ctx.report(TextRange::at(start, TextSize::of(text)), message);
                }
                Some(_) => {}
            }
        }
    }
}
