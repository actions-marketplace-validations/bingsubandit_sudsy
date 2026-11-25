//! The check `first-word` of the family `doc-comment`: the first line of the doc comment of a
//! declaration starts with the name of the declaration, as in Go `// Name does ...`. A
//! declaration without a doc comment is left to `missing-doc-comment`. Params: the doc comment
//! and the declarations as of `missing-doc-comment` ([`crate::doc_run::Docs`],
//! [`crate::doc_comments::Targets`]), `message` with `{name}` and `{expected}` (the text the
//! line must start with), and per group of declarations:
//! - `prefix.<group>`: text before the name (`Package `, for the comment before a package clause);
//! - `articles.<group>`: words that may stand before the name (`A An The`);
//! - `form-in-group.<group>`: true when the declarations of a group in parentheses take the
//!   group's comment as their own (Go types); otherwise the comment of such a group is not read;
//! - `detached.<group>`: true to report a comment that has the right form but is parted from the
//!   declaration by a blank line, with the param `detached-message`, at the start of the line
//!   after the comment.
//!
//! The report is at the first comment of the doc comment.
//!
//! concept: doc-first-word
//! what: The doc comment of a declaration starts with the declaration's name.
//! tools: revive exported, package-comments
//! rules: doc-first-word, package-comment-form
//! reads: family:doc-comment

use crate::doc_comments::{Hit, Targets};
use crate::doc_run::Docs;
use crate::{Params, Rule, RuleContext, RuleMeta};
use core_syntax::{SyntaxNode, TextRange, TextSize};

struct FirstWord {
    meta: &'static RuleMeta,
    message: String,
    detached: String,
    targets: Targets,
    docs: Docs,
}

pub(crate) fn build(meta: &'static RuleMeta, mut p: Params) -> Result<Box<dyn Rule>, String> {
    let message = p.text("message")?;
    let detached = p.optional("detached-message").unwrap_or_default();
    let docs = Docs::read(&mut p)?;
    if !docs.lines() {
        return Err("the check `first-word` needs the param `doc-lines`".to_string());
    }
    let targets = Targets::read(&mut p)?;
    Ok(Box::new(FirstWord {
        meta,
        message,
        detached,
        targets,
        docs,
    }))
}

impl FirstWord {
    /// A comment of the right form that a blank line parts from the declaration.
    fn detached(&self, hit: &Hit<'_, '_>, ctx: &mut RuleContext<'_>) {
        let expected = format!("{}{} ", hit.group.prefix, hit.name.text);
        let g = ctx.file.grammar;
        if let Some(end) = self.docs.detached(g, &hit.holder, &expected) {
            let text = ctx.file.text;
            let line = text[usize::from(end)..]
                .find('\n')
                .map_or(text.len(), |i| usize::from(end) + i + 1);
            let at = TextSize::new(line.min(text.len()) as u32);
            ctx.report(TextRange::new(at, at), self.detached.clone());
        }
    }
}

impl Rule for FirstWord {
    fn meta(&self) -> &'static RuleMeta {
        self.meta
    }

    fn bodies(&self) -> bool {
        false
    }

    fn tags(&self) -> &[&'static str] {
        &self.targets.tags
    }

    fn check(&self, node: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        let g = ctx.file.grammar;
        let Some(hit) = self.targets.select(g, node, ctx.file.text) else {
            return;
        };
        let Some(doc) = self.docs.of(g, &hit.holder) else {
            if hit.group.detached {
                self.detached(&hit, ctx);
            }
            return;
        };
        let grouped = self.docs.in_open_group(g, &hit.holder);
        if grouped && !hit.group.form_in_group && self.docs.group_has_doc(g, &hit.holder) {
            return;
        }
        let name = hit.name.text.as_ref();
        let mut expected = format!("{}{name}", hit.group.prefix);
        let article = (hit.group.articles.iter())
            .find(|a| a.as_str() != name && doc.line.starts_with(&format!("{a} ")));
        if let Some(a) = article {
            expected = format!("{a} {expected}");
        }
        if doc.line.starts_with(&format!("{expected} ")) {
            return;
        }
        let message = (self.message.replace("{name}", name)).replace("{expected}", &expected);
        ctx.report(doc.first.text_range(), message);
    }
}
