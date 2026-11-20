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

use crate::diagnostic::Edit;
use crate::{Family, Params, Rule, RuleContext, RuleMeta};
use core_config::{OptionValue, ResolvedOptions};
use core_format::case::Cases;
use core_syntax::{Grammar, SyntaxKind, TextRange};

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

/// The token kinds of the `[case]` row whose policy is the option `option`.
fn row_kinds(grammar: &Grammar, ctx: &RuleContext<'_>, option: &str) -> Vec<SyntaxKind> {
    ctx.section("case")
        .filter_map(|row| row.split_once(" : "))
        .filter(|(_, policy)| policy.split(" of ").next().map(str::trim) == Some(option))
        .flat_map(|(kinds, _)| kinds.split('|'))
        .filter_map(|name| grammar.kind_by_name(name.trim()))
        .collect()
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
        let kinds = ctx.parsed(self.meta.id, || row_kinds(file.grammar, ctx, &self.option));
        if kinds.is_empty() {
            return;
        }
        // The other rows of the file's rows keep the policy `consistent` (no option, no row).
        let mut options = ResolvedOptions::default();
        options.set(
            &self.option,
            OptionValue::String(ctx.options.string("policy").to_string()),
        );
        let mut recase = cases.recaser(&options);
        for token in file
            .root
            .descendants_with_tokens()
            .filter_map(|e| e.into_token())
            .filter(|t| kinds.contains(&t.kind()))
        {
            if let Some(fixed) = recase.word(token.kind(), token.text()) {
                let message =
                    (self.message.replace("{name}", token.text())).replace("{fixed}", &fixed);
                let range: TextRange = token.text_range();
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
