//! The family `doc-param-missing`: a parameter of a documented declaration that its doc comment
//! does not name. Params:
//! - `declarations`: declaration rows ([`crate::decl_rows`]) of what has parameters (the node
//!   with a child tagged `parameter-list` whose children tagged `parameter` are the parameters);
//! - `name`: the slot path of the name in a parameter;
//! - `param`: a regex whose first group is the name a doc comment gives to a parameter
//!   (`<param\s+name="([^"]*)"` for XML comments, `@param\s+(\w+)` for Javadoc);
//! - `skip-doc` (optional): a regex of the doc comments that are left out (`<inheritdoc`);
//! - the visibility params of [`crate::doc_comments`] (`visible`, ...): only visible declarations
//!   in visible types are checked;
//! - `doc-start` as in [`crate::doc_comments`]; `message` with `{name}`.
//!
//! A declaration without a doc comment is not reported (that is `missing-doc-comment`). The
//! report is at the parameter.
//!
//! concept: doc-param-model
//! what: A parameter of a documented declaration has an entry in its doc comment.
//! tools: StyleCop SA1611, Checkstyle JavadocMethod
//! rules: doc-param-missing
//! reads: family:doc-param-missing, tag:parameter-list, tag:parameter

use crate::decl_rows::Row;
use crate::doc_comments::{Visibility, holder, leading_doc_text, words};
use crate::slot_path::SlotPath;
use crate::{Family, Params, Rule, RuleContext, RuleMeta};
use crate::{Meta, Re, meta_regex};
use core_syntax::SyntaxNode;

pub const FAMILY: Family = Family {
    name: "doc-param-missing",
    build,
};

struct Missing {
    meta: &'static RuleMeta,
    message: String,
    rows: Vec<Row>,
    tags: Vec<&'static str>,
    name: SlotPath,
    param: Meta,
    skip: Option<Re>,
    doc_start: Vec<String>,
    vis: Visibility,
}

fn build(meta: &'static RuleMeta, mut p: Params) -> Result<Box<dyn Rule>, String> {
    let message = p.text("message")?;
    let rows = (p.text("declarations")?.split('|'))
        .map(|alt| Row::parse("declarations", alt))
        .collect::<Result<Vec<_>, _>>()?;
    let name = SlotPath::parse(&p.text("name")?)?;
    let param = meta_regex(&p.text("param")?).map_err(|e| format!("param `param`: {e}"))?;
    let skip = match p.optional("skip-doc") {
        Some(s) => Some(Re::lazy(&s).map_err(|e| format!("param `skip-doc`: {e}"))?),
        None => None,
    };
    let vis = Visibility::read(&mut p)?;
    let doc_start = words(&p.optional("doc-start").unwrap_or_default());
    p.done()?;
    let mut tags: Vec<&'static str> = Vec::new();
    for r in &rows {
        if !tags.contains(&r.tag.as_str()) {
            tags.push(Box::leak(r.tag.clone().into_boxed_str()));
        }
    }
    Ok(Box::new(Missing {
        meta,
        message,
        rows,
        tags,
        name,
        param,
        skip,
        doc_start,
        vis,
    }))
}

impl Rule for Missing {
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
        if !self.rows.iter().any(|r| r.matches(g, node)) {
            return;
        }
        let holder = holder(g, node);
        if !self.vis.visible_declaration(g, node, &holder)
            || !self.vis.enclosing_visible(g, node, self.vis.words())
        {
            return;
        }
        let Some(list) = node
            .children()
            .find(|c| g.has_tag(c.kind(), "parameter-list"))
        else {
            return;
        };
        let Some(doc) = leading_doc_text(&holder, g, &self.doc_start) else {
            return;
        };
        if self.skip.as_ref().is_some_and(|s| s.is_match(&doc)) {
            return;
        }
        let documented: Vec<&str> = self
            .param
            .captures_iter(&doc)
            .filter_map(|c| c.get_group(1).map(|m| doc[m.start..m.end].trim()))
            .collect();
        for parameter in list.children().filter(|c| g.has_tag(c.kind(), "parameter")) {
            let Some(name) = self.name.find(g, &parameter, ctx.file.text) else {
                continue;
            };
            if !documented.contains(&&*name.text) {
                ctx.report(name.range, self.message.replace("{name}", &name.text));
            }
        }
    }
}
