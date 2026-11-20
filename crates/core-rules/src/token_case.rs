//! The `token-case` family: the words of the token kinds in the `[case]` rows of the language's
//! `format.rules` are written in one case (`core_format::case`). The param `option` names the
//! row (the formatter option that holds its policy, `keyword_case`); the rule's own option
//! `policy` (`consistent`, `upper`, `lower`, `capitalise`) says which case the words must have,
//! and the fix writes the word in it. The message has `{name}` (the word) and `{fixed}` (the
//! word as it must be written).
//!
//! concept: token-case-lint
//! what: Words of the token kinds of a `[case]` row are not written in the case the rule asks for.
//! tools: sqlfluff CP01-CP05
//! reads: family:token-case, rules:case

use crate::checks::FileCache;
use crate::diagnostic::Edit;
use crate::{Family, Params, Rule, RuleContext, RuleMeta};
use core_format::case::Cases;
use core_syntax::{SyntaxKind, TextRange};

pub const FAMILY: Family = Family {
    name: "token-case",
    build,
};

struct TokenCase {
    meta: &'static RuleMeta,
    message: String,
    option: String,
}

fn build(meta: &'static RuleMeta, mut p: Params) -> Result<Box<dyn Rule>, String> {
    let rule = TokenCase {
        meta,
        message: p.text("message")?,
        option: p.text("option")?,
    };
    p.done()?;
    Ok(Box::new(rule))
}

thread_local! {
    /// The tokens of the last file whose kind has a `[case]` row: the five rules of a language
    /// share one walk.
    static TOKENS: FileCache<Vec<(SyntaxKind, TextRange)>> = const { FileCache::new() };
}

impl Rule for TokenCase {
    fn meta(&self) -> &'static RuleMeta {
        self.meta
    }

    fn bodies(&self) -> bool {
        false
    }

    fn check_file(&self, file: &crate::FileContext<'_>, ctx: &mut RuleContext<'_>) {
        let cases = ctx.parsed("token-case", || {
            Cases::parse(file.grammar, ctx.section("case"))
        });
        let kinds = cases.kinds_of(&self.option);
        if kinds.is_empty() {
            return;
        }
        let tokens = TOKENS.with(|c| {
            c.get(file, || {
                (file.root.descendants_with_tokens())
                    .filter_map(|e| e.into_token())
                    .filter(|t| cases.covers(t.kind()))
                    .map(|t| (t.kind(), t.text_range()))
                    .collect()
            })
        });
        let mut recase = cases.recaser_of(&self.option, ctx.options.string("policy"));
        for &(kind, range) in tokens.iter().filter(|(k, _)| kinds.contains(k)) {
            let text = &file.text[range];
            if let Some(fixed) = recase.word(kind, text) {
                let message = (self.message.replace("{name}", text)).replace("{fixed}", &fixed);
                ctx.report_fix(
                    range,
                    message,
                    (
                        "write the word in that case",
                        vec![Edit::replace(range, fixed)],
                    ),
                );
            }
        }
    }
}
