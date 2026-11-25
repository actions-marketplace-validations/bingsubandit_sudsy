//! The checks of directory rules over the facts of one unit (the files of a directory): a
//! declaration nothing uses, a need nothing provides, and a requirement the unit never states.
//! The language's `collect` writes the facts (key: the name, value: one of the kinds below) and its
//! `finish` calls the check. Every check is silent when the unit may be incomplete
//! (`ProjectContext::complete`): an absence is only a finding when every file was read.
//!
//! concept: directory-facts
//! what: A declaration no file of the directory uses, a need no file provides, a requirement the directory never states, a directory with no doc comment.
//! tools: tflint terraform_unused_declarations, terraform_required_providers, terraform_required_version, revive package-comments

use crate::project::{Fact, ProjectContext, ProjectFacts};
use std::collections::HashSet;

/// A name the unit declares; reported when no fact of kind `USED` has its key.
pub const DECLARED: &str = "declared";
pub const USED: &str = "used";
/// A name a file needs; reported (last one of the name) when no fact of kind `PROVIDED` has its key.
pub const NEEDED: &str = "needed";
pub const PROVIDED: &str = "provided";
/// A place to report a requirement that no file of the unit states: reported when no fact of kind
/// `NEEDED` or `PROVIDED` has its key.
pub const ANCHOR: &str = "anchor";

fn of_kind<'a>(
    facts: &'a ProjectFacts,
    rule: &'a str,
    kind: &'a str,
) -> impl Iterator<Item = &'a Fact> {
    facts.of_rule(rule).filter(move |f| f.value == kind)
}

fn keys<'a>(facts: &'a ProjectFacts, rule: &'a str, kinds: &[&str]) -> HashSet<&'a str> {
    facts
        .of_rule(rule)
        .filter(|f| kinds.contains(&f.value.as_str()))
        .map(|f| f.key.as_str())
        .collect()
}

/// Reports each `DECLARED` fact whose key no `USED` fact has: `message` with `{name}`.
pub fn unused(facts: &ProjectFacts, ctx: &mut ProjectContext<'_>, message: &str) {
    if !ctx.complete {
        return;
    }
    let rule = ctx.rule_id();
    let used = keys(facts, rule, &[USED]);
    for f in of_kind(facts, rule, DECLARED).filter(|f| !used.contains(f.key.as_str())) {
        ctx.report(&f.file, f.range, message.replace("{name}", &f.key));
    }
}

/// Reports, for each key of the `NEEDED` facts that no `PROVIDED` fact has, the last `NEEDED`
/// fact of the key; and each `ANCHOR` whose key has neither kind: `message` with `{name}`.
pub fn missing(facts: &ProjectFacts, ctx: &mut ProjectContext<'_>, message: &str) {
    if !ctx.complete {
        return;
    }
    let rule = ctx.rule_id();
    let provided = keys(facts, rule, &[PROVIDED]);
    let mut last: Vec<&Fact> = Vec::new();
    for f in of_kind(facts, rule, NEEDED).filter(|f| !provided.contains(f.key.as_str())) {
        match last.iter().position(|l| l.key == f.key) {
            Some(at) => last[at] = f,
            None => last.push(f),
        }
    }
    let stated = keys(facts, rule, &[NEEDED, PROVIDED]);
    let anchors = of_kind(facts, rule, ANCHOR).filter(|f| !stated.contains(f.key.as_str()));
    for f in last.into_iter().chain(anchors) {
        ctx.report(&f.file, f.range, message.replace("{name}", &f.key));
    }
}

/// Reports one `ANCHOR` fact of the unit (the one in the first file by name) when some fact of
/// kind `NEEDED` and none of kind `PROVIDED` exists (any key). `message` with `{name}` (the key).
pub fn first_anchor(facts: &ProjectFacts, ctx: &mut ProjectContext<'_>, message: &str) {
    if !ctx.complete {
        return;
    }
    let rule = ctx.rule_id();
    if of_kind(facts, rule, NEEDED).next().is_none()
        || of_kind(facts, rule, PROVIDED).next().is_some()
    {
        return;
    }
    let file_name = |f: &Fact| {
        let name = f.file.file_name().and_then(|n| n.to_str());
        name.unwrap_or_default().to_string()
    };
    if let Some(f) = of_kind(facts, rule, ANCHOR).min_by_key(|f| file_name(f)) {
        ctx.report(&f.file, f.range, message.replace("{name}", &f.key));
    }
}
