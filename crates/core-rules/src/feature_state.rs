//! `feature-not-enabled`: a token of a kind of the option `kinds` whose text matches the regex of the
//! option `use` is met while the feature is off. The feature starts off; a word of the option `on`
//! (a whole word, ignoring case) turns it on, one of `off` turns it off; the file is read in order.
//! Reported on the token.
//!
//! concept: feature-not-enabled
//! what: A token that needs a feature is met in a file, or a part of it, where the feature is off.
//! tools: blinter P008
//! rules: feature-not-enabled
//! reads: rule:feature-not-enabled

use crate::options::{Check, Opt, OptionDecl};
use crate::rule;
use crate::rule::{FileContext, RuleContext};
use crate::sorting::cached_regex;

rule! {
    pub FeatureNotEnabled {
        id: "feature-not-enabled",
        category: Suspicious,
        default: Off,
        description: "Reports a token of the kinds in the option `kinds` that matches the regex of \
              the option `use` while a feature is off. The feature is off at the start of the \
              file, on after a word of the option `on`, and off after a word of `off` (words \
              compared as whole words, ignoring case).",
        why: "Text that needs a feature is read as plain text while the feature is off, so the \
              command gets the notation instead of its value.\n\
              To fix, turn the feature on before the use, or write the plain form.",
        example: "echo !name!   // reported: delayed expansion was never turned on",
        options: [
        OptionDecl {
            name: "kinds",
            default: Opt::List(&[]),
            check: Check::Any,
            description: "The token kinds that can use the feature.",
        },
        OptionDecl {
            name: "use",
            default: Opt::Str(""),
            check: Check::Regex,
            description: "A regex the text of a token that uses the feature matches.",
        },
        OptionDecl {
            name: "on",
            default: Opt::List(&[]),
            check: Check::Any,
            description: "The words that turn the feature on.",
        },
        OptionDecl {
            name: "off",
            default: Opt::List(&[]),
            check: Check::Any,
            description: "The words that turn the feature off.",
        }],
    }

    fn check_file(&self, file: &FileContext<'_>, ctx: &mut RuleContext<'_>) {
        let Some(uses) = cached_regex(ctx.options.string("use")).filter(|_| !ctx.options.string("use").is_empty())
        else {
            return;
        };
        let (g, text) = (file.grammar, file.text);
        let mut on = false;
        for t in file.root.descendants_with_tokens().filter_map(|e| e.into_token()) {
            let word = t.text();
            let end = usize::from(t.text_range().end());
            let whole = text[end..].chars().next().is_none_or(|c| c.is_whitespace() || c == ')');
            if whole && ctx.options.list_has("on", word, true) {
                on = true;
            } else if whole && ctx.options.list_has("off", word, true) {
                on = false;
            } else if !on && ctx.options.list_has("kinds", g.name(t.kind()), false) && uses.is_match(word) {
                ctx.report(t.text_range(), format!("`{}` needs a feature that is off here", word.trim()));
            }
        }
    }
}
