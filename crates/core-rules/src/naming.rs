//! The `naming-convention` family: the name of a kind of declaration follows the convention an
//! option sets. Each param but `message` and `match` is `<option>: <rows>`: the rows are
//! `|`-separated declaration rows ([`crate::decl_rows`]: a grammar tag, `<parent-tag`, `&modifier`,
//! `&@annotation`, `=path`; the node's `name` slot, or its `key` slot, by default). The
//! option's value is a convention (`PascalCase`, `camelCase`, `snake_case`, `UPPER_CASE`, `words`:
//! [`crate::word_case`], with the words of the option `initialisms` in capitals) or `off`;
//! leading `$` and `_` are ignored (not by `words`). A path ending in `:each` names every
//! part it finds (`a, b int`); when a node matches several rows of one option, all of them are
//! read. A name that matches the option `ignore_pattern` (when the rule has one) is left out. With `match: regex` the value is a regular expression
//! the name must match instead (empty: not checked; a value that starts with `!` is a
//! regular expression the name must not match). The message has `{name}` and
//! `{convention}` (the value) and, for `words`, `{suggestion}` (the name spelled right); a param `message.<option>` is the message of that option.
//!
//! concept: naming-convention
//! what: The name of a kind of declaration follows the convention an option sets.
//! rules: naming-convention
//! reads: family:naming-convention

use crate::decl_rows::Row;
use crate::slot_path::SlotPath;
use crate::word_case;
use crate::{Family, Params, RegexCache, Rule, RuleContext, RuleMeta};
use core_syntax::{SyntaxNode, TextRange};

pub const FAMILY: Family = Family {
    name: "naming-convention",
    build,
};

/// Whether `name` follows `convention`; an unknown convention (`off`) accepts every name.
pub fn follows(convention: &str, name: &str) -> bool {
    let name = name.trim_start_matches(['$', '_']);
    let first = name.chars().next();
    let plain = name.chars().all(|c| c.is_ascii_alphanumeric());
    match convention {
        "PascalCase" => plain && first.is_some_and(|c| c.is_ascii_uppercase()),
        "camelCase" => plain && first.is_some_and(|c| c.is_ascii_lowercase()),
        "snake_case" => !name.chars().any(|c| c.is_ascii_uppercase()),
        "UPPER_CASE" => !name.chars().any(|c| c.is_ascii_lowercase()),
        _ => true,
    }
}

struct Naming {
    meta: &'static RuleMeta,
    message: String,
    /// `message.<option>` params: the message of one option.
    messages: Vec<(String, String)>,
    regex: Option<RegexCache>,
    /// The compiled `ignore_pattern`s.
    ignored: RegexCache,
    /// Rows with the most conditions first.
    rows: Vec<Row>,
    tags: Vec<&'static str>,
}

fn build(meta: &'static RuleMeta, mut p: Params) -> Result<Box<dyn Rule>, String> {
    let message = p.text("message")?;
    let regex = p.choice("match", false, &[("convention", false), ("regex", true)])?;
    let mut rows = Vec::new();
    let mut messages = Vec::new();
    for (option, value) in p.rest() {
        if let Some(option) = option.strip_prefix("message.") {
            messages.push((option.to_string(), value));
            continue;
        }
        for alt in value.split('|') {
            rows.push(Row::parse(&option, alt)?);
        }
    }
    rows.sort_by_key(|r| std::cmp::Reverse(r.conditions()));
    let mut tags: Vec<&'static str> = Vec::new();
    for r in &rows {
        if !tags.contains(&r.tag.as_str()) {
            tags.push(Box::leak(r.tag.clone().into_boxed_str()));
        }
    }
    Ok(Box::new(Naming {
        meta,
        message,
        messages,
        regex: regex.then(RegexCache::default),
        ignored: RegexCache::default(),
        rows,
        tags,
    }))
}

impl Rule for Naming {
    fn meta(&self) -> &'static RuleMeta {
        self.meta
    }

    fn bodies(&self) -> bool {
        false
    }

    fn tags(&self) -> &[&'static str] {
        &self.tags
    }

    fn check(&self, node: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        let g = ctx.file.grammar;
        // a more specific row whose option is empty (regex mode: not checked) leaves the node to the next row
        let regex = self.regex.is_some();
        let Some(first) = (self.rows.iter())
            .find(|r| r.matches(g, node) && !(regex && ctx.options.string(&r.option).is_empty()))
        else {
            return;
        };
        let convention = ctx.options.string(&first.option);
        let ignore = ctx.options.string("ignore_pattern");
        let initialisms = if convention == "words" {
            ctx.options.list("initialisms")
        } else {
            Vec::new()
        };
        let default = SlotPath::parse("name|key").unwrap_or_else(|_| unreachable!());
        let mut seen: Vec<TextRange> = Vec::new();
        for row in &self.rows {
            if row.option != first.option || !row.matches(g, node) {
                continue;
            }
            let path = row.path.as_ref().unwrap_or(&default);
            for found in path.find_each(g, node, ctx.file.text) {
                if seen.contains(&found.range) {
                    continue;
                }
                seen.push(found.range);
                if !ignore.is_empty() && self.ignored.is_match(ignore, &found.text) {
                    continue;
                }
                let (text, suggestion) = match &self.regex {
                    Some(cache) => {
                        // `!` in front: the name must not match
                        let (forbid, pattern) = match convention.strip_prefix('!') {
                            Some(p) => (true, p),
                            None => (false, convention),
                        };
                        (
                            &*found.text,
                            (!pattern.is_empty() && cache.is_match(pattern, &found.text) == forbid)
                                .then(String::new),
                        )
                    }
                    None if convention == "words" => {
                        (&*found.text, word_case::suggest(&found.text, &initialisms))
                    }
                    None => {
                        let text = found.text.trim_start_matches('$');
                        (text, (!follows(convention, text)).then(String::new))
                    }
                };
                if let Some(suggestion) = suggestion {
                    let template = (self.messages.iter())
                        .find(|(o, _)| *o == row.option)
                        .map_or(&self.message, |(_, m)| m);
                    let message = template
                        .replace("{name}", text)
                        .replace("{suggestion}", &suggestion);
                    ctx.report(found.range, message.replace("{convention}", convention));
                }
            }
        }
    }
}
