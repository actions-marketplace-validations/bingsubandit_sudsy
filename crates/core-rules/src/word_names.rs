//! Names that collide with the language's words, asked of the language through `RuleHooks::word`
//! and `RuleHooks::fold` (its keyword tables, per dialect). Family `keyword-name`: an unquoted
//! name (the token kind the param `kind` names) written like a keyword of the dialect. Family
//! `needless-quotes`: a quoted name (param `kind`) that needs no quotes: it is a plain word, not
//! reserved, and already in the case the dialect folds unquoted names to. The message of both has
//! `{name}`.
//!
//! concept: word-names
//! what: Unquoted names that are keywords and quoted names that need no quotes, from the language's word tables.
//! tools: sqlfluff RF04, RF06
//! reads: rule:keyword-identifier, rule:needless-quotes

use crate::checks::FileCache;
use crate::{Family, Fold, Params, Rule, RuleContext, RuleMeta, Word};
use core_syntax::TextRange;

pub const KEYWORD_NAME: Family = Family {
    name: "keyword-name",
    build: |meta, p| build(meta, p, Check::Keyword),
};

pub const NEEDLESS_QUOTES: Family = Family {
    name: "needless-quotes",
    build: |meta, p| build(meta, p, Check::Quotes),
};

#[derive(Clone, Copy)]
enum Check {
    Keyword,
    Quotes,
}

struct WordName {
    meta: &'static RuleMeta,
    message: String,
    kind: String,
    /// The kinds of node a name may sit directly in (empty: anywhere).
    parents: Vec<String>,
    check: Check,
}

fn build(meta: &'static RuleMeta, mut p: Params, check: Check) -> Result<Box<dyn Rule>, String> {
    let rule = WordName {
        meta,
        message: p.text("message")?,
        kind: p.text("kind")?,
        parents: (p
            .optional("parents")
            .unwrap_or_default()
            .split(',')
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect()),
        check,
    };
    p.done()?;
    Ok(Box::new(rule))
}

thread_local! {
    /// The name tokens of the last file, one cache per check.
    static KEYWORDS: FileCache<Vec<TextRange>> = const { FileCache::new() };
    static QUOTED: FileCache<Vec<TextRange>> = const { FileCache::new() };
}

/// A word that needs no quotes: letters, digits and underscores, a letter in it, no digit first.
fn plain(inner: &str) -> bool {
    let b = inner.as_bytes();
    !b.is_empty()
        && !b[0].is_ascii_digit()
        && b.iter().all(|c| c.is_ascii_alphanumeric() || *c == b'_')
        && b.iter().any(u8::is_ascii_alphabetic)
}

impl Rule for WordName {
    fn meta(&self) -> &'static RuleMeta {
        self.meta
    }

    fn bodies(&self) -> bool {
        false
    }

    fn check_file(&self, file: &crate::FileContext<'_>, ctx: &mut RuleContext<'_>) {
        let Some(kind) = file.grammar.kind_by_name(&self.kind) else {
            return;
        };
        let parents: Vec<_> = (self.parents.iter())
            .filter_map(|n| file.grammar.kind_by_name(n))
            .collect();
        let walk = |c: &FileCache<Vec<TextRange>>| {
            c.get(file, || {
                (file.root.descendants_with_tokens())
                    .filter_map(|e| e.into_token())
                    .filter(|t| t.kind() == kind)
                    .filter(|t| {
                        parents.is_empty()
                            || t.parent().is_some_and(|p| parents.contains(&p.kind()))
                    })
                    .map(|t| t.text_range())
                    .collect()
            })
        };
        let tokens = match self.check {
            Check::Keyword => KEYWORDS.with(walk),
            Check::Quotes => QUOTED.with(walk),
        };
        let version = file.hooks.language_version();
        let fold = file.hooks.fold(version);
        for &range in tokens.iter() {
            let text = &file.text[range];
            let flagged = match self.check {
                Check::Keyword => {
                    text.len() > 1
                        && Word::of(file.hooks, version, &text.to_ascii_uppercase())
                            == Word::Keyword
                }
                Check::Quotes => {
                    let inner = text.get(1..text.len().saturating_sub(1)).unwrap_or("");
                    plain(inner)
                        && Word::of(file.hooks, version, &inner.to_ascii_uppercase())
                            != Word::Reserved
                        && match fold {
                            Fold::Upper => inner == inner.to_ascii_uppercase(),
                            Fold::Lower => inner == inner.to_ascii_lowercase(),
                            Fold::None => true,
                        }
                }
            };
            if flagged {
                ctx.report(range, self.message.replace("{name}", text));
            }
        }
    }
}
