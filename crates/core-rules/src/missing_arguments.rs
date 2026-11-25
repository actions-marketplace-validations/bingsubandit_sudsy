//! The `missing-arguments` family: a call of a known command that leaves every one of its
//! mandatory parameters unbound. The language passes its table of [`Signature`]s to [`build`] (a
//! command's positional parameters in order and its mandatory ones). Params of the rule: `name-token` (the kind of the `-name` token of an argument),
//! `pipe` (the kind of the token that puts a command behind another) and `splat` (the prefix of a
//! splatted argument). The message has `{name}`.
//!
//! A `-name` takes the element after it as its value (a switch too), and the element after a
//! `name:value` pair as well; the other elements fill the positional parameters in order, the
//! ones given by name skipped. A command behind a pipe or with a splat is not looked at.
//!
//! concept: missing-arguments
//! what: A call of a known command that binds none of its mandatory parameters, from a table of command signatures.
//! tools: PSScriptAnalyzer PSUseCmdletCorrectly
//! reads: path:signature::Signature

use crate::tree::is_code;
use crate::{Params, Rule, RuleContext, RuleMeta};
use core_syntax::signature::Signature;
use core_syntax::{SyntaxElement, SyntaxNode};

struct Missing {
    meta: &'static RuleMeta,
    message: String,
    name_token: String,
    pipe: String,
    splat: String,
    commands: &'static [Signature],
}

/// The rule for the commands of the table; the family's params are the message and the tokens.
pub fn build(
    table: &'static [Signature],
    meta: &'static RuleMeta,
    mut p: Params,
) -> Result<Box<dyn Rule>, String> {
    let message = p.text("message")?;
    let name_token = p.text("name-token")?;
    let pipe = p.text("pipe")?;
    let splat = p.text("splat")?;
    Ok(Box::new(Missing {
        meta,
        message,
        name_token,
        pipe,
        splat,
        commands: table,
    }))
}

impl Rule for Missing {
    fn meta(&self) -> &'static RuleMeta {
        self.meta
    }

    fn tags(&self) -> &[&'static str] {
        &["call"]
    }

    fn bodies(&self) -> bool {
        false
    }

    fn check(&self, node: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        let g = ctx.file.grammar;
        let mut tokens = node
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .filter(|t| is_code(t, g));
        let Some(callee) = tokens.next() else { return };
        let Some(signature) =
            (self.commands.iter()).find(|c| c.name.eq_ignore_ascii_case(callee.text()))
        else {
            return;
        };
        if signature.mandatory.is_empty() || self.behind_pipe(node, ctx) {
            return;
        }
        let (mut named, mut positional) = (Vec::new(), 0usize);
        let mut elements = node
            .children_with_tokens()
            .filter(|e| match e {
                SyntaxElement::Token(t) => is_code(t, g),
                SyntaxElement::Node(_) => true,
            })
            .skip(1)
            .peekable();
        while let Some(element) = elements.next() {
            let SyntaxElement::Token(t) = &element else {
                positional += 1;
                continue;
            };
            if t.text().starts_with(&self.splat) {
                return;
            }
            if g.name(t.kind()) != self.name_token {
                positional += 1;
                continue;
            }
            named.push(t.text().trim_start_matches('-').to_ascii_lowercase());
            let attached = elements
                .next_if(|e| matches!(e, SyntaxElement::Token(c) if c.text() == ":"))
                .is_some();
            // The value, and after a `name:value` pair the next element too.
            for _ in 0..1 + usize::from(attached) {
                if elements
                    .next_if(|e| !matches!(e, SyntaxElement::Token(c) if g.name(c.kind()) == self.name_token))
                    .is_none()
                {
                    break;
                }
            }
        }
        let is_named = |m: &str| named.iter().any(|n| n.eq_ignore_ascii_case(m));
        let free = signature.positional.iter().filter(|p| !is_named(p));
        let by_position: Vec<&&str> = free.take(positional).collect();
        let bound = |m: &&str| is_named(m) || by_position.contains(&m);
        if !signature.mandatory.iter().any(bound) {
            let message = self.message.replace("{name}", callee.text());
            ctx.report(callee.text_range(), message);
        }
    }
}

impl Missing {
    /// Does a pipe token stand right before the command?
    fn behind_pipe(&self, node: &SyntaxNode, ctx: &RuleContext<'_>) -> bool {
        let g = ctx.file.grammar;
        let mut previous = node.prev_sibling_or_token();
        while let Some(e) = previous {
            match &e {
                SyntaxElement::Token(t) if !is_code(t, g) => previous = e.prev_sibling_or_token(),
                SyntaxElement::Token(t) => return g.name(t.kind()) == self.pipe,
                SyntaxElement::Node(_) => return false,
            }
        }
        false
    }
}
