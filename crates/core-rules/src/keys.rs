//! Rules on the keys of mappings, written against the `#mapping` and `#key-value` grammar tags and
//! the language's [`RuleHooks`](crate::hooks::RuleHooks): `no-duplicate-keys`, `no-empty-keys` and
//! `key-name-casing`.
//!
//! concept: no-duplicate-keys, no-empty-keys, key-name-casing
//! what: Rules on the keys of mappings.
//! rules: no-duplicate-keys, no-empty-keys, key-name-casing
//! reads: rule:no-duplicate-keys

use crate::hooks::Accessor;
use crate::names;
use crate::options::{Check, Opt, OptionDecl};
use crate::rule;
use crate::rule::RuleContext;
use crate::sorting;
use crate::tree::significant_range;
use core_syntax::{Grammar, SyntaxElement, SyntaxNode, TextRange};
use std::borrow::Cow;
use std::collections::HashMap;

/// The entries of a `#mapping` node that have a key, with their keys.
pub fn entries<'a>(
    mapping: &SyntaxNode,
    grammar: &'a Grammar,
) -> impl Iterator<Item = (SyntaxNode, SyntaxElement)> + 'a {
    mapping
        .children()
        .filter(|c| grammar.has_tag(c.kind(), "key-value"))
        .filter_map(|entry| {
            let key = grammar.slot(&entry, "key")?;
            Some((entry, key))
        })
}

/// [`entries`] collected, for the rules that index into them.
pub fn keyed(mapping: &SyntaxNode, grammar: &Grammar) -> Vec<(SyntaxNode, SyntaxElement)> {
    entries(mapping, grammar).collect()
}

/// Where a key is reported: its token, or its node without the comments around it.
pub fn key_range(key: &SyntaxElement, grammar: &Grammar) -> TextRange {
    match key {
        SyntaxElement::Node(node) => significant_range(node, grammar),
        SyntaxElement::Token(token) => token.text_range(),
    }
}

rule! {
    pub NoDuplicateKeys {
        id: "no-duplicate-keys",
        category: Correctness,
        default: Error,
        description: "Reports a key that a mapping has twice, after decoding escapes and quotes (`{\"a\": \
              1, \"\\u0061\": 2}` has `a` twice). It reads the mappings of the language's grammar: \
              JSON and JSONC objects, YAML and TOML mappings, JavaScript object literals, and \
              similar entries of other languages. A YAML merge key (`<<`) may repeat, and the \
              option `merge_keys: forbid` reports a second one too. The second key is reported, \
              the first is shown as related.",
        why: "Parsers disagree on what a duplicate means: many keep the last value, some the \
              first, some reject the file. The value that is lost is silent, so a file can mean \
              something different in two tools, and the second entry is often a copy that was \
              meant to have another name.\n\
              To fix, remove one of the keys, or give it its own name.",
        example: "{ \"a\": 1, \"a\": 2 }   // reported at the second \"a\"",
        options: [OptionDecl {
            name: "merge_keys",
            default: Opt::Str("allow"),
            check: Check::OneOf(&["allow", "forbid"]),
            description: "Languages with a merge key (YAML `<<`) only. `allow` skips merge keys, \
                      which may repeat; `forbid` reports a second one like any other duplicate key.",
        }],
        tags: &["mapping"],
    }

    fn check(&self, node: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        let (grammar, hooks, text) = (ctx.file.grammar, ctx.file.hooks, ctx.file.text);
        let merge_key = grammar
            .keys()
            .merge
            .as_deref()
            .filter(|_| ctx.options.string("merge_keys") == "allow");
        // Ranges are found only for the keys that are reported (finding one reads the tree), so
        // the first occurrence is kept as its index and looked up again on a duplicate.
        let flat = hooks.flat_keys(node);
        if flat.is_some() || !grammar.keys().accessors.is_empty() {
            return custom_keys(node, flat, ctx, merge_key);
        }
        let mut seen = Seen::default();
        for (index, (_, key)) in entries(node, grammar).enumerate() {
            let name = hooks.key_name(grammar, &key, text);
            if merge_key == Some(&*name) || !hooks.is_name(grammar, &key, text) {
                continue;
            }
            if let Some((first, name)) = seen.first(name, index) {
                let first = entries(node, grammar)
                    .nth(first)
                    .map_or_else(|| key_range(&key, grammar), |(_, k)| key_range(&k, grammar));
                ctx.report(key_range(&key, grammar), format!("duplicate key `{name}`"))
                    .related(first, "first defined here");
            }
        }
    }
}

/// `no-duplicate-keys` for a language with flat or accessor entries.
fn custom_keys(
    node: &SyntaxNode,
    flat: Option<Vec<SyntaxElement>>,
    ctx: &mut RuleContext<'_>,
    merge_key: Option<&str>,
) {
    let (grammar, hooks, text) = (ctx.file.grammar, ctx.file.hooks, ctx.file.text);
    let accessors = !grammar.keys().accessors.is_empty();
    let mut seen = Seen::default();
    // name -> (kinds seen: 1 plain, 2 getter, 4 setter; index of the first), with accessors only
    let mut kinds: HashMap<Cow<'_, str>, (u8, usize)> = HashMap::new();
    let walked = flat
        .is_none()
        .then(|| entries(node, grammar))
        .into_iter()
        .flatten();
    let listed = flat.iter().flatten().map(|k| (node.clone(), k.clone()));
    let first_range = |first: usize, key: &SyntaxElement| {
        let at = flat.as_ref().and_then(|f| f.get(first).cloned());
        let at = at.or_else(|| entries(node, grammar).nth(first).map(|(_, k)| k));
        key_range(at.as_ref().unwrap_or(key), grammar)
    };
    for (index, (entry, key)) in listed.chain(walked).enumerate() {
        let name = hooks.key_name(grammar, &key, text);
        if merge_key == Some(&*name) || !hooks.is_name(grammar, &key, text) {
            continue;
        }
        let repeat = if accessors {
            let kind = match names::accessor(grammar, &entry) {
                None => 1,
                Some(Accessor::Get) => 2,
                Some(Accessor::Set) => 4,
            };
            let slot = kinds.entry(name.clone()).or_insert((0, index));
            let clash = if kind == 1 {
                slot.0 != 0
            } else {
                slot.0 & (kind | 1) != 0
            };
            slot.0 |= kind;
            clash.then_some((slot.1, name))
        } else {
            seen.first(name, index)
        };
        if let Some((first, name)) = repeat {
            let first = first_range(first, &key);
            ctx.report(key_range(&key, grammar), format!("duplicate key `{name}`"))
                .related(first, "first defined here");
        }
    }
}

const SMALL: usize = 8;

/// The entry that first used each name: a linear scan for the few entries most mappings have, a
/// hash map beyond that (hashing and growing a map cost more than comparing a handful of names).
#[derive(Default)]
struct Seen<'a> {
    small: [(Cow<'a, str>, usize); SMALL],
    len: usize,
    map: HashMap<Cow<'a, str>, usize>,
}

impl<'a> Seen<'a> {
    /// The index of the earlier entry named `name`, with the name handed back; records `name` as
    /// `index` when there is none.
    fn first(&mut self, name: Cow<'a, str>, index: usize) -> Option<(usize, Cow<'a, str>)> {
        if self.map.is_empty() {
            if let Some((_, first)) = self.small[..self.len].iter().find(|(n, _)| *n == name) {
                let first = *first;
                return Some((first, name));
            }
            if self.len < SMALL {
                self.small[self.len] = (name, index);
                self.len += 1;
                return None;
            }
            let small = self.small.iter_mut().map(|(n, i)| (std::mem::take(n), *i));
            self.map.extend(small);
        }
        if let Some(&first) = self.map.get(&name) {
            return Some((first, name));
        }
        self.map.insert(name, index);
        None
    }
}

rule! {
    pub NoEmptyKeys {
        id: "no-empty-keys",
        category: Suspicious,
        default: Off,
        description: "Reports a key that is empty or only white space after decoding escapes (`\"\": 4`).",
        why: "A property with no name cannot be reached the usual ways: a dotted path, a command \
              line option or an environment variable has no spelling for it, and some consumers \
              fail on it. It is nearly always a missing name.\n\
              To fix, give the key a name.",
        example: "{ \"\": 1, \" \": 2 }   // both keys reported",
        tags: &["mapping"],
    }

    fn check(&self, node: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        let (grammar, hooks, text) = (ctx.file.grammar, ctx.file.hooks, ctx.file.text);
        for (_, key) in entries(node, grammar) {
            if hooks.key_name(grammar, &key, text).trim().is_empty() {
                ctx.report(key_range(&key, grammar), "empty key");
            }
        }
    }
}

const CONVENTIONS: [&str; 5] = [
    "camelCase",
    "PascalCase",
    "snake_case",
    "kebab-case",
    "SCREAMING_SNAKE_CASE",
];

fn has_symbol(c: char) -> bool {
    matches!(c, '\u{21}'..='\u{23}' | '\u{25}'..='\u{2c}' | '.' | '/' | '\u{3a}'..='\u{40}'
        | '\u{5b}'..='\u{5e}' | '`' | '\u{7b}'..='\u{7d}')
}

fn matches_convention(convention: &str, key: &str) -> bool {
    if key.chars().any(|c| has_symbol(c) || c.is_whitespace()) {
        return false;
    }
    let upper = key.chars().any(|c| c.is_ascii_uppercase());
    let lower = key.chars().any(|c| c.is_ascii_lowercase());
    let separators = key.contains(['-', '_']);
    let first = key.chars().next();
    match convention {
        "camelCase" => !first.is_some_and(|c| c.is_ascii_uppercase()) && !separators,
        "PascalCase" => !first.is_some_and(|c| c.is_ascii_lowercase()) && !separators,
        "snake_case" => !upper && !key.contains('-') && !key.contains("__"),
        "kebab-case" => {
            !upper && !key.starts_with('-') && !key.contains('_') && !key.contains("--")
        }
        "SCREAMING_SNAKE_CASE" => !lower && !key.contains('-') && !key.contains("__"),
        _ => false,
    }
}

rule! {
    pub KeyNameCasing {
        id: "key-name-casing",
        category: Style,
        default: Off,
        description: "Reports a key that does not follow one of the allowed naming conventions of the \
              option `allowed` (default `camelCase`; also `PascalCase`, `snake_case`, \
              `kebab-case`, `SCREAMING_SNAKE_CASE`). A key with punctuation or white space never \
              matches; `camelCase` also rejects `_` and `-`, and `snake_case` and `kebab-case` \
              reject capitals. Keys that match one of the `ignored_keys` regexes, and a YAML \
              merge key (`<<`), are not checked. There is no fix, because renaming a key changes \
              the data.",
        why: "Configuration and data files are read by programs and people, and one casing for \
              all keys lets both guess a key's name without looking it up. A key in another \
              casing is often a typo or a copy from another file with other conventions.\n\
              To fix, rename the key and every place that reads it.",
        example: "{ \"first_name\": 1 }   // reported with the default `camelCase`",
        options: [
            OptionDecl {
                name: "allowed",
                default: Opt::List(&["camelCase"]),
                check: Check::OneOf(&CONVENTIONS),
                description: "Accepted conventions: `camelCase`, `PascalCase`, `snake_case`, \
                          `kebab-case`, `SCREAMING_SNAKE_CASE`. A key is fine when it matches one.",
            },
            sorting::IGNORED_KEYS,
    ],
        tags: &["mapping"],
    }

    fn check(&self, node: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        let (grammar, hooks, text) = (ctx.file.grammar, ctx.file.hooks, ctx.file.text);
        let allowed = ctx.options.list("allowed");
        let ignored: Vec<_> = ctx
            .options
            .list("ignored_keys")
            .into_iter()
            .filter_map(sorting::cached_regex)
            .collect();
        if allowed.is_empty() {
            return;
        }
        let merge_key = grammar.keys().merge.as_deref();
        for (_, key) in entries(node, grammar) {
            if merge_key.is_some() && merge_key == Some(&*hooks.key_name(grammar, &key, text)) {
                continue; // a merge key is no key of the data
            }
            let parts = names::parts_of(grammar, &key, text);
            let whole;
            let names: &[Cow<'_, str>] = match &parts {
                Some(parts) => parts,
                None => {
                    whole = [hooks.key_name(grammar, &key, text)];
                    &whole
                }
            };
            let Some(name) = names.iter().find(|name| {
                !ignored.iter().any(|r| r.is_match(name))
                    && !allowed.iter().any(|c| matches_convention(c, name))
            }) else {
                continue;
            };
            ctx.report(
                key_range(&key, grammar),
                format!(
                    "key `{name}` must match one of the following formats: {}",
                    allowed.join(", ")
                ),
            );
        }
    }
}
