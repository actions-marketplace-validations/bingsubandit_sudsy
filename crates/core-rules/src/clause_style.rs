//! The `clause-style` family: the items of a list clause (`GROUP BY`, `ORDER BY`) are written one
//! way in the whole file, by name or by position. Params: `clauses` (the clause node kinds),
//! `explicit` (the node kinds of an item written by name or expression), `implicit` (the token
//! kinds of an item written by position). A clause that mixes the two, or that differs from the
//! first clause of the file, is reported.
//!
//! concept: clause-style
//! what: The items of the list clauses of a file are all written by name or all by position.
//! tools: sqlfluff AM06
//! reads: rule:group-order-style

use crate::{Family, Params, Rule, RuleContext, RuleMeta};
use core_syntax::SyntaxKind;

pub const FAMILY: Family = Family {
    name: "clause-style",
    build,
};

struct ClauseStyle {
    meta: &'static RuleMeta,
    message: String,
    clauses: Vec<String>,
    explicit: Vec<String>,
    implicit: Vec<String>,
}

fn list(p: &mut Params, key: &str) -> Result<Vec<String>, String> {
    Ok(p.text(key)?.split(',').map(str::to_string).collect())
}

fn build(meta: &'static RuleMeta, mut p: Params) -> Result<Box<dyn Rule>, String> {
    let rule = ClauseStyle {
        meta,
        message: p.text("message")?,
        clauses: list(&mut p, "clauses")?,
        explicit: list(&mut p, "explicit")?,
        implicit: list(&mut p, "implicit")?,
    };
    p.done()?;
    Ok(Box::new(rule))
}

impl Rule for ClauseStyle {
    fn meta(&self) -> &'static RuleMeta {
        self.meta
    }

    fn bodies(&self) -> bool {
        false
    }

    fn check_file(&self, file: &crate::FileContext<'_>, ctx: &mut RuleContext<'_>) {
        let g = file.grammar;
        let kinds = |names: &[String]| -> Vec<SyntaxKind> {
            names.iter().filter_map(|n| g.kind_by_name(n)).collect()
        };
        let (clauses, explicit, implicit) = (
            kinds(&self.clauses),
            kinds(&self.explicit),
            kinds(&self.implicit),
        );
        let mut prior: Option<bool> = None;
        for clause in ctx.nodes(&clauses) {
            let (mut by_name, mut by_position) = (false, false);
            for e in clause.children_with_tokens() {
                match e {
                    rowan::NodeOrToken::Node(n) => by_name |= explicit.contains(&n.kind()),
                    rowan::NodeOrToken::Token(t) => by_position |= implicit.contains(&t.kind()),
                }
            }
            if !(by_name || by_position) {
                continue;
            }
            let mixed = by_name && by_position;
            let differs = !mixed && prior.is_some_and(|p| p != by_name);
            if prior.is_none() && !mixed {
                prior = Some(by_name);
            }
            if mixed || differs {
                let range = crate::tree::significant_range(&clause, g);
                ctx.report(range, self.message.clone());
            }
        }
    }
}
