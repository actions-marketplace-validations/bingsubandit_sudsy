//! The `multiline-start` family: an expression that spans lines starts on a line of its own, not
//! after code on the line before it (`val a = run {`, `f(g {`). Params:
//! - `kind` (words): the kind names of the nodes the rule looks at; `Name=Token` takes the
//!   expression to be the child node after the node's first child token or node `Token`
//!   (`Property=Eq`), `Name=Token:text` only when that child's text is `text` (`=` for an
//!   assignment), a bare `Name` the node itself.
//!
//! The report is on the expression's first line.
//!
//! concept: multiline-start-family
//! what: Reports a multi-line expression that starts on a line that has code before it.
//! tools: ktlint multiline-expression-wrapping

use crate::{Family, FileContext, Params, Rule, RuleContext, RuleMeta};
use core_syntax::{SyntaxElement, TextRange};

pub const FAMILY: Family = Family {
    name: "multiline-start",
    build,
};

struct MultilineStart {
    meta: &'static RuleMeta,
    message: String,
    kinds: Vec<(String, Option<String>)>,
}

fn words(p: &mut Params, key: &str) -> Vec<String> {
    p.optional(key)
        .map(|s| s.split_whitespace().map(str::to_string).collect())
        .unwrap_or_default()
}

fn build(meta: &'static RuleMeta, mut p: Params) -> Result<Box<dyn Rule>, String> {
    let rule = MultilineStart {
        meta,
        message: p.text("message")?,
        kinds: words(&mut p, "kind")
            .into_iter()
            .map(|k| match k.split_once('=') {
                Some((n, t)) => (n.to_string(), Some(t.to_string())),
                None => (k, None),
            })
            .collect(),
    };
    if rule.kinds.is_empty() {
        return Err("param `kind` is missing".into());
    }
    p.done()?;
    Ok(Box::new(rule))
}

/// Is the child of kind `kind` and text `text` the one the expression follows?
fn matches(after: &Option<String>, kind: &str, text: &str) -> bool {
    let Some(spec) = after else {
        return false;
    };
    match spec.split_once(':') {
        Some((k, t)) => k == kind && t == text,
        None => spec == kind,
    }
}

impl Rule for MultilineStart {
    fn meta(&self) -> &'static RuleMeta {
        self.meta
    }

    fn check_file(&self, file: &FileContext<'_>, ctx: &mut RuleContext<'_>) {
        let g = file.grammar;
        for node in file.root.descendants() {
            let name = g.name(node.kind());
            let Some((_, after)) = self.kinds.iter().find(|(k, _)| k == name) else {
                continue;
            };
            let range = if after.is_none() {
                crate::tree::significant_range(&node, g)
            } else {
                let mut seen = false;
                let mut found = None;
                for e in node.children_with_tokens() {
                    match e {
                        SyntaxElement::Token(t) if matches(after, g.name(t.kind()), t.text()) => {
                            seen = true;
                        }
                        SyntaxElement::Node(n)
                            if !seen
                                && matches(
                                    after,
                                    g.name(n.kind()),
                                    &file.text[crate::tree::significant_range(&n, g)],
                                ) =>
                        {
                            seen = true;
                        }
                        SyntaxElement::Node(n) if seen => {
                            found = Some(crate::tree::significant_range(&n, g));
                            break;
                        }
                        _ => {}
                    }
                }
                match found {
                    Some(r) => r,
                    None => continue,
                }
            };
            let text = &file.text[range];
            if !text.contains('\n') {
                continue;
            }
            let line = file.lines.line(range.start());
            let (start, _) = file.lines.line_range(file.text, line);
            let before = &file.text[start..usize::from(range.start())];
            if before.trim().is_empty() {
                continue;
            }
            let end = range.start() + core_syntax::TextSize::of(text.lines().next().unwrap_or(""));
            ctx.report(TextRange::new(range.start(), end), self.message.clone());
        }
    }
}
