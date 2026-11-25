//! Rules that apply to every language. A language-specific rule cannot reuse a shared rule's name.
//! Those on keys and sequences ([`crate::keys`], [`crate::sorted`]) act on the grammar tags
//! `#mapping` and `#sequence` (`no-duplicate-sections`: `#sections`), so they do nothing in a
//! language that does not tag them.
//!
//! concept: none (glue: registration of the shared rules)

use crate::comments::{
    BlockCommentAfterOpen, CommentSpace, NoEmptyComment, NoSingleLineBlockComment,
};
use crate::keys::{KeyNameCasing, NoDuplicateKeys, NoEmptyKeys};
use crate::options::{Check, Opt, OptionDecl};
use crate::rule;
use crate::rule::{FileContext, Rule, RuleContext};
use crate::sections::NoDuplicateSections;
use crate::sorted::{SortedArrayValues, SortedKeys};
use core_syntax::{TextRange, TextSize};

pub static RULES: &[&dyn Rule] = &[
    &MaxLineLength,
    &NoTrailingWhitespace,
    &CommentSpace,
    &NoEmptyComment,
    &NoSingleLineBlockComment,
    &BlockCommentAfterOpen,
    &NoDuplicateKeys,
    &NoEmptyKeys,
    &NoDuplicateSections,
    &KeyNameCasing,
    &SortedKeys,
    &SortedArrayValues,
    &crate::suppress::UnknownRuleInDirective,
    &crate::modifiers::ModifierOrder,
    &crate::curly::Curly,
    &crate::redundant_return_type::RedundantReturnType,
    &crate::heading_style::HeadingStyle,
    &crate::hard_tabs::NoHardTabs,
    &crate::declared_name::DeclaredNamePattern,
    &crate::documented_command::DocumentedCommand,
    &crate::statement_blank_lines::StatementBlankLines,
    &crate::statement_indent::StatementIndent,
];

rule! {
    MaxLineLength {
        id: "max-line-length",
        category: Style,
        default: Off,
        description: "Reports a line longer than `max` characters (default 120). With \
              `allow_non_breakable_words` (default on) a line that has no space after its \
              indentation, a leading `#` run or `- ` is not reported (a long URL); \
              `allow_non_breakable_inline_mappings` also allows a YAML `key: value` line whose \
              value has no space.",
        why: "Long lines do not fit next to each other in a diff or a code review, wrap \
              unpredictably in narrow windows, and hide the end of the line when a view scrolls \
              sideways. The limit is a convention of the project, set with `max`.\n\
              To fix, break the line at a natural place.",
        example: "max-line-length: { level: warn, options: { max: 100 } }\n\
              A line of 101 characters is reported from the 101st character on.",
        options: [
            OptionDecl {
                name: "max",
                default: Opt::Int(120),
                check: Check::Any,
                description: "Longest allowed line, in characters.",
            },
            OptionDecl {
                name: "allow_non_breakable_words",
                default: Opt::Bool(true),
                check: Check::Any,
                description: "Do not report a line that has no space after its indentation (and \
                          after a leading `#` run or `- `), such as a long URL.",
            },
            OptionDecl {
                name: "allow_non_breakable_inline_mappings",
                default: Opt::Bool(false),
                check: Check::Any,
                description: "With `allow_non_breakable_words`, also do not report a `key: value` \
                          line whose value has no space. Only in languages that have such lines \
                          (YAML).",
            },
    ],
    }

    fn check_file(&self, file: &FileContext<'_>, ctx: &mut RuleContext<'_>) {
        let max = usize::try_from(ctx.options.int("max")).unwrap_or(usize::MAX);
        let non_breakable = ctx.options.bool("allow_non_breakable_words");
        let inline_mappings = ctx.options.bool("allow_non_breakable_inline_mappings");
        for line in 0..file.lines.line_count() {
            let (start, end) = file.lines.line_range(file.text, line);
            let text = &file.text[start..end];
            let Some((offset, _)) = text.char_indices().nth(max) else {
                continue;
            };
            let range = TextRange::new(
                TextSize::from((start + offset) as u32),
                TextSize::from(end as u32),
            );
            if non_breakable
                && (is_non_breakable(text) || (inline_mappings && file.hooks.inline_mapping_line(text)))
            {
                continue;
            }
            let length = text.chars().count();
            ctx.report(
                range,
                format!("line is {length} characters long; the limit is {max}"),
            );
        }
    }
}

rule! {
    NoTrailingWhitespace {
        id: "no-trailing-whitespace",
        category: Style,
        default: Off,
        description: "Reports spaces or tabs at the end of a line, from the first one to the line break. \
              Tokens of the kinds the option `ignore_kinds` names (Markdown's fenced code) are not \
              looked at, and `hard_break_spaces` allows Markdown's two-space line break.",
        why: "Trailing white space cannot be seen, adds nothing, and many editors strip it, so a \
              later edit shows a change on a line the author did not touch and makes diffs and \
              blame noisy.\n\
              To fix, delete it.",
        example: "text   \n   // reported at the spaces",
        options: [
            OptionDecl {
                name: "ignore_kinds",
                default: Opt::List(&[]),
                check: Check::Any,
                description: "Kinds of tokens whose trailing white space is not reported (the code of a fence).",
            },
            OptionDecl {
                name: "hard_break_spaces",
                default: Opt::Int(0),
                check: Check::Any,
                description: "With a value above 0, that many spaces or more at the end of a line that the next non-blank line follows are a hard line break and not reported (Markdown: 2).",
            },
        ],
    }

    fn check_file(&self, file: &FileContext<'_>, ctx: &mut RuleContext<'_>) {
        // the options, and the language's `rule` row of this rule
        let mut kinds = ctx.options.list("ignore_kinds");
        let mut break_spaces = ctx.options.int("hard_break_spaces") as usize;
        if let Some(row) = file.grammar.rule_row("no-trailing-whitespace") {
            for (name, values) in &row.options {
                match name.as_str() {
                    "ignore_kinds" => kinds.extend(values.iter().map(String::as_str)),
                    "hard_break_spaces" => {
                        break_spaces = values.first().and_then(|v| v.parse().ok()).unwrap_or(0)
                    }
                    _ => {}
                }
            }
        }
        let skipped = crate::tree::token_ranges(file.root, file.grammar, kinds.as_slice());
        for line in 0..file.lines.line_count() {
            let (start, end) = file.lines.line_range(file.text, line);
            let kept = file.text[start..end].trim_end_matches([' ', '\t']).len();
            if start + kept < end {
                let range = TextRange::new(
                    TextSize::from((start + kept) as u32),
                    TextSize::from(end as u32),
                );
                // a hard line break: enough spaces, and the paragraph goes on in the next line
                let hard_break = break_spaces > 0
                    && end - (start + kept) >= break_spaces
                    && file.text[start + kept..end].bytes().all(|b| b == b' ')
                    && line + 1 < file.lines.line_count()
                    && {
                        let (a, b) = file.lines.line_range(file.text, line + 1);
                        !file.text[a..b].trim().is_empty()
                    };
                if !hard_break && !skipped.iter().any(|s| s.contains_range(range)) {
                    ctx.report(range, "trailing whitespace");
                }
            }
        }
    }
}

/// Does the line have no space after its indentation, a leading `#` run and the character after
/// it, or a leading `-` and the character after it? (yamllint's `allow-non-breakable-words`.)
fn is_non_breakable(line: &str) -> bool {
    let body = line.trim_start_matches(' ');
    let skip = |s: &str| s.chars().skip(1).collect::<String>();
    let rest = match body.chars().next() {
        None => return false,
        Some('#') => skip(body.trim_start_matches('#')),
        Some('-') => body.chars().skip(2).collect(),
        Some(_) => return !body.contains(' '),
    };
    !rest.contains(' ')
}
