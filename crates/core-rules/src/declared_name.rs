//! `declared-name-pattern`: the names a flat command list declares match a pattern by what they
//! declare. A row of `names` (`command:position:class`, position `N`, `-N` or `N+`) says which
//! arguments of which command are names and their class; the class `variable` is settled per
//! command: a word of `markers` (`WORD:class`, the last found wins) among the arguments gives the
//! class, else the innermost block of `scopes` (`block:class`) the command stands in, else the
//! class of the row with no block. `patterns` (`class:regex`, matched from the start of the name)
//! say what each class must match; a class without a pattern is not checked.
//!
//! concept: declared-name-pattern
//! what: The names a command declares match a pattern by their class: argument position, marker word or enclosing block.
//! tools: cmake-lint C0103
//! rules: declared-name-pattern
//! reads: path:block_pair::Block

use crate::options::{Check, Opt, OptionDecl};
use crate::rule;
use crate::rule::{FileContext, RuleContext};
use crate::sorting::cached_regex;
use core_syntax::block_pair::Pairing;
use core_syntax::{SyntaxElement, SyntaxNode};

rule! {
    pub DeclaredNamePattern {
        id: "declared-name-pattern",
        category: Style,
        default: Off,
        description: "A name that a command declares (a variable it sets, a function or its \
                  parameters) does not match the pattern of its class. Only for languages whose \
                  commands open and close blocks.",
        example: "function(f)\nset(Local 1)   // reported when `local` wants lower case\nendfunction()",
        options: [
            OptionDecl {
                name: "names",
                default: Opt::List(&[]),
                check: Check::Any,
                description: "Rows `command:position:class` (`command` may be `command FIRST-ARGUMENT`): the argument at the position (`0`, `-1` for the last, or `1+` for the rest) are names of the class; the class `variable` is told by `markers` and `scopes`.",
            },
            OptionDecl {
                name: "markers",
                default: Opt::List(&[]),
                check: Check::Any,
                description: "Rows `WORD:class`: a command with the argument WORD (or the arguments `WORD WORD`, in a row) declares a variable of the class; the last row that fits wins.",
            },
            OptionDecl {
                name: "scopes",
                default: Opt::List(&[]),
                check: Check::Any,
                description: "Rows `block:class`: a variable set inside the block that starts with the command `block` (the innermost listed one) has the class; the row with no block is the class outside them. An empty class is not checked.",
            },
            OptionDecl {
                name: "patterns",
                default: Opt::List(&[]),
                check: Check::Any,
                description: "Rows `class:regex`: the regex a name of the class must match from its start.",
            },
            OptionDecl {
                name: "reference_start",
                default: Opt::Str(""),
                check: Check::Any,
                description: "Where a reference to another name starts inside a name (`${` in CMake); the references, up to the matching `reference_end`, are cut out before the name is matched, and a name that is only references is left out.",
            },
            OptionDecl {
                name: "reference_end",
                default: Opt::Str(""),
                check: Check::Any,
                description: "Where a reference ends (`}` in CMake).",
            },
            OptionDecl {
                name: "ignore_pattern",
                default: Opt::Str(""),
                check: Check::Regex,
                description: "A regex; a name that matches is left out.",
            },
        ],
    }

    fn check_file(&self, file: &FileContext<'_>, ctx: &mut RuleContext<'_>) {
        let blocks = file.hooks.blocks();
        if blocks.is_empty() {
            return;
        }
        let names = rows(ctx.options.list("names"));
        let markers = rows(ctx.options.list("markers"));
        let scopes = rows(ctx.options.list("scopes"));
        let patterns = rows(ctx.options.list("patterns"));
        let ignore = ctx.options.string("ignore_pattern");
        let (start, end) = (ctx.options.string("reference_start"), ctx.options.string("reference_end"));
        let g = file.grammar;
        // the range of an argument without the trivia around it
        let range_of = |n: &SyntaxNode| {
            let mut own = n
                .descendants_with_tokens()
                .filter_map(|e| e.into_token())
                .filter(|t| !g.is_trivia(t.kind()))
                .map(|t| t.text_range());
            let first = own.next().unwrap_or_else(|| n.text_range());
            first.cover(own.last().unwrap_or(first))
        };
        let text_of = |n: &SyntaxNode| {
            let r = range_of(n);
            &file.text[usize::from(r.start())..usize::from(r.end())]
        };
        let mut pairing = Pairing::new(blocks);
        for command in file.root.children() {
            let Some(SyntaxElement::Token(word)) = g.slot(&command, "name") else {
                continue;
            };
            let word = word.text();
            // the block the command opens does not hold its own names
            let scoped = scope(&scopes, &pairing);
            pairing.word(word, 0);
            let Some(SyntaxElement::Node(list)) = g.slot(&command, "arguments") else {
                continue;
            };
            let args: Vec<SyntaxNode> = g.slots(&list, "items").filter_map(|e| e.into_node()).collect();
            let texts: Vec<&str> = args.iter().map(text_of).collect();
            for (_, rest) in names.iter().filter(|(c, _)| starts(c, word, &texts)) {
                let Some((position, class)) = rest.split_once(':') else {
                    continue;
                };
                let from = position.trim_end_matches('+').parse::<i64>().unwrap_or(0);
                let from = if from < 0 {
                    args.len().saturating_sub(from.unsigned_abs() as usize)
                } else {
                    from as usize
                };
                let last = if position.ends_with('+') { args.len() } else { from + 1 };
                for arg in args.iter().take(last).skip(from) {
                    let class = if class == "variable" {
                        (markers.iter().rev())
                            .find(|(w, _)| follow(w, &texts))
                            .map_or(scoped, |(_, class)| *class)
                    } else {
                        class
                    };
                    let Some((_, pattern)) = patterns.iter().find(|(c, _)| *c == class) else {
                        continue;
                    };
                    let text = text_of(arg);
                    if !ignore.is_empty() && cached_regex(ignore).is_some_and(|re| re.is_match(text)) {
                        continue;
                    }
                    let bare = cut_references(text, start, end);
                    if bare.is_empty() {
                        continue;
                    }
                    let anchored = cached_regex(&format!("^(?:{pattern})"));
                    if anchored.is_some_and(|re| !re.is_match(&bare)) {
                        ctx.report(
                            range_of(arg),
                            format!("{class} name {text} does not match `{pattern}`"),
                        );
                    }
                }
            }
        }
    }
}

/// Whether the command `word` with the arguments `texts` is the one a row names: `command` or
/// `command FIRST-ARGUMENT` (case ignored).
fn starts(row: &str, word: &str, texts: &[&str]) -> bool {
    let mut parts = row.split_whitespace();
    parts.next().is_some_and(|c| c.eq_ignore_ascii_case(word))
        && parts
            .next()
            .is_none_or(|a| texts.first().is_some_and(|t| t.eq_ignore_ascii_case(a)))
}

/// Whether the words of `marker` (separated by blanks) are arguments in a row.
fn follow(marker: &str, texts: &[&str]) -> bool {
    let words: Vec<&str> = marker.split_whitespace().collect();
    !words.is_empty() && texts.windows(words.len()).any(|w| w == words.as_slice())
}

/// `a:b` rows split at the first colon.
fn rows(list: Vec<&str>) -> Vec<(&str, &str)> {
    list.into_iter().filter_map(|r| r.split_once(':')).collect()
}

/// The class of a variable set where `pairing` stands: the innermost listed block, else the row
/// with no block.
fn scope<'a>(scopes: &[(&'a str, &'a str)], pairing: &Pairing) -> &'a str {
    let inner = pairing
        .enclosing()
        .find_map(|w| scopes.iter().find(|(b, _)| b.eq_ignore_ascii_case(w)));
    inner
        .or_else(|| scopes.iter().find(|(b, _)| b.is_empty()))
        .map_or("", |(_, class)| *class)
}

/// `text` without the references that start with `start` and end with the matching `end`
/// (nested ones included); an unclosed one runs to the end. Without `start`, the text itself.
fn cut_references(text: &str, start: &str, end: &str) -> String {
    if start.is_empty() || end.is_empty() {
        return text.to_string();
    }
    let mut out = String::new();
    let mut depth = 0;
    let mut rest = text;
    while !rest.is_empty() {
        if rest.starts_with(start) {
            depth += 1;
            rest = &rest[start.len()..];
        } else if depth > 0 && rest.starts_with(end) {
            depth -= 1;
            rest = &rest[end.len()..];
        } else {
            let c = rest.chars().next().unwrap_or_default();
            if depth == 0 {
                out.push(c);
            }
            rest = &rest[c.len_utf8()..];
        }
    }
    out
}
