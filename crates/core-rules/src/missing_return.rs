//! The family `missing-return`: a function that declares a return type and whose end can be
//! reached without a `return`, on the flow graph of its body (`core_flow::build`). The functions are
//! the `#function` nodes with a `body` slot (a bodiless one is abstract); the type is the node of the
//! slot `return_type` (a leading `:` or `->` is left out) or, without one, the type that the
//! doc comment names. A generator (a `#yield` in its own body) returns by itself. Params:
//! - `skip-types`: the types that need no value, words (`void never mixed`); a union with one of them is skipped;
//! - `skip-names` (optional): functions that return nothing by nature, words (`__construct`);
//! - `doc` (optional): a regex whose first group is the return type a doc comment gives
//!   (`@return\s+(.+)`: the type is cut at the first space outside brackets); `doc-start` as in [`crate::doc_comments`];
//! - `message` with `{type}`.
//!
//! The report is at the last statement of the body.
//!
//! concept: missing-return
//! what: A function with a declared return type whose end is reachable without a return.
//! tools: phpstan return.missing, typescript TS2366
//! rules: missing-return
//! reads: family:missing-return, tag:function

use crate::doc_comments::{holder, leading_doc_text, words};
use crate::tree::significant_range;
use crate::{Family, Params, Rule, RuleContext, RuleMeta};
use crate::{Meta, meta_regex};
use core_flow::build as flow_of;
use core_syntax::{Grammar, SyntaxElement, SyntaxNode};

pub const FAMILY: Family = Family {
    name: "missing-return",
    build,
};

struct Missing {
    meta: &'static RuleMeta,
    message: String,
    skip: Vec<String>,
    skip_names: Vec<String>,
    doc: Option<Meta>,
    doc_start: Vec<String>,
}

fn build(meta: &'static RuleMeta, mut p: Params) -> Result<Box<dyn Rule>, String> {
    let message = p.text("message")?;
    let skip = words(&p.text("skip-types")?);
    let skip_names = words(&p.optional("skip-names").unwrap_or_default());
    let doc = match p.optional("doc") {
        Some(d) => Some(meta_regex(&d).map_err(|e| format!("param `doc`: {e}"))?),
        None => None,
    };
    let doc_start = words(&p.optional("doc-start").unwrap_or_default());
    p.done()?;
    Ok(Box::new(Missing {
        meta,
        message,
        skip,
        skip_names,
        doc,
        doc_start,
    }))
}

/// The type at the start of `text`: up to the first space outside brackets (`array{int, bool}|null a list`).
fn type_prefix(text: &str) -> &str {
    let mut depth = 0i32;
    for (i, c) in text.char_indices() {
        match c {
            '(' | '[' | '{' | '<' => depth += 1,
            ')' | ']' | '}' | '>' => depth -= 1,
            c if c.is_whitespace() && depth <= 0 => return &text[..i],
            _ => {}
        }
    }
    text
}

/// Does the body of `n` yield (nested functions left out)?
fn yields(n: &SyntaxNode, g: &Grammar) -> bool {
    let mut stack: Vec<SyntaxNode> = n.children().collect();
    while let Some(c) = stack.pop() {
        if g.has_tag(c.kind(), "yield") {
            return true;
        }
        if !g.has_tag(c.kind(), "function") {
            stack.extend(c.children());
        }
    }
    false
}

impl Missing {
    /// The declared return type of `n`: the native one, else the one of its doc comment.
    fn declared(&self, n: &SyntaxNode, ctx: &RuleContext<'_>) -> Option<(String, bool)> {
        let g = ctx.file.grammar;
        if let Some(e) = g.slot(n, "return_type") {
            let range = match &e {
                SyntaxElement::Node(t) => significant_range(t, g),
                SyntaxElement::Token(t) => t.text_range(),
            };
            let text =
                ctx.file.text[range].trim_start_matches([':', '-', '>', ' ', '\t', '\n', '\r']);
            return Some((text.to_string(), false));
        }
        let doc = leading_doc_text(&holder(g, n), g, &self.doc_start)?;
        let found = self.doc.as_ref()?.captures_iter(&doc).next()?;
        let m = found.get_group(1)?;
        Some((type_prefix(&doc[m.start..m.end]).to_string(), true))
    }
}

impl Rule for Missing {
    fn meta(&self) -> &'static RuleMeta {
        self.meta
    }

    fn bodies(&self) -> bool {
        false
    }

    fn tags(&self) -> &[&'static str] {
        &["function"]
    }

    fn check(&self, node: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        let g = ctx.file.grammar;
        let Some(SyntaxElement::Node(body)) = g.slot(node, "body") else {
            return;
        };
        if let Some(SyntaxElement::Node(name)) = g.slot(node, "name") {
            let name = ctx.file.text[significant_range(&name, g)].to_string();
            if self
                .skip_names
                .iter()
                .any(|s| s.eq_ignore_ascii_case(&name))
            {
                return;
            }
        } else if let Some(SyntaxElement::Token(name)) = g.slot(node, "name") {
            let name = &ctx.file.text[name.text_range()];
            if self.skip_names.iter().any(|s| s.eq_ignore_ascii_case(name)) {
                return;
            }
        }
        let Some((declared, from_doc)) = self.declared(node, ctx) else {
            return;
        };
        // a native type is a union of names; a doc type can nest them (`array{int, bool}|null`,
        // `($a is null ? static : void)`): a skipped word anywhere in it counts
        let skipped = |m: &str| self.skip.iter().any(|s| s.eq_ignore_ascii_case(m.trim()));
        if declared.is_empty()
            || if from_doc {
                declared
                    .split(|c: char| !c.is_alphanumeric() && c != '_')
                    .any(skipped)
            } else {
                declared.split('|').any(skipped)
            }
        {
            return;
        }
        if yields(node, g) {
            return;
        }
        if let Some(flow) = flow_of(g, ctx.file.text, node)
            && flow.reach().block[flow.end]
        {
            let at = body.children().last().unwrap_or(body);
            let range = significant_range(&at, g);
            ctx.report(range, self.message.replace("{type}", &declared));
        }
    }
}
