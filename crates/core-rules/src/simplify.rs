//! The `simplify` family: statements and expressions that say in several words what fewer say,
//! and jumps with nothing to jump out of, read from the grammar's roles (`#conditional` with its
//! `condition`, `then` and `else` fields, `#branch` for the `elif` clauses of a chain, `#return`,
//! `#break`, `#continue`, `#loop`, `#function`, `#unary`, `#binary`, `#literal`). Params:
//! - `check`: `collapsible-if` (an `if` whose only statement is an `if`, neither with a further
//!   branch), `needless-bool` (`return` of a boolean in each branch), `if-with-same-arms` (two
//!   neighbouring branches with the same body), `boolean-conditional` (a ternary with the boolean
//!   literals `then` and `else`; the params are `true` or `false`), `twisted-conditional`
//!   (`b if not a else a`), `negated-comparison` (`not a == b`; param `operator` is the operator
//!   whose negation is reported, `except-in` names the functions where it is left alone and
//!   `except-guard: true` leaves a test whose `if` only throws), `double-negation`,
//!   `redundant-cast` (`!!x` or a call of one of the names in `calls` where a boolean is expected
//!   anyway), `jump-outside` (param `jump`: `break`, `continue` or `return`; `inside` lists the
//!   tags a `break` may be in besides `loop`).
//! - the message has `{name}` where the check passes a name.

use crate::tree::significant_range;
use crate::{Edit, Family, Params, Rule, RuleContext, RuleMeta};
use core_syntax::roles::{code_text, outer, un};
use core_syntax::{Grammar, SyntaxNode, TextRange};

pub const FAMILY: Family = Family {
    name: "simplify",
    build,
};

#[derive(Clone)]
enum Check {
    CollapsibleIf,
    NeedlessBool,
    SameArms,
    BooleanConditional(bool, bool),
    TwistedConditional,
    NegatedComparison {
        operator: String,
        except_in: Vec<String>,
        guard: bool,
    },
    DoubleNegation,
    RedundantCast(Vec<String>),
    Jump {
        jump: &'static str,
        inside: Vec<String>,
    },
}

struct Simplify {
    meta: &'static RuleMeta,
    message: String,
    check: Check,
    tags: &'static [&'static str],
}

fn words(p: &mut Params, key: &str) -> Vec<String> {
    p.optional(key)
        .map(|s| s.split_whitespace().map(str::to_string).collect())
        .unwrap_or_default()
}

fn build(meta: &'static RuleMeta, mut p: Params) -> Result<Box<dyn Rule>, String> {
    let name = p.text("check")?;
    let flag = |p: &mut Params, k: &str| p.choice(k, true, &[("true", true), ("false", false)]);
    let (check, tags): (Check, &'static [&'static str]) = match name.as_str() {
        "collapsible-if" => (Check::CollapsibleIf, &["conditional"]),
        "needless-bool" => (Check::NeedlessBool, &["conditional"]),
        "if-with-same-arms" => (Check::SameArms, &["conditional"]),
        "boolean-conditional" => {
            let (a, b) = (flag(&mut p, "then")?, flag(&mut p, "else")?);
            (Check::BooleanConditional(a, b), &["conditional"])
        }
        "twisted-conditional" => (Check::TwistedConditional, &["conditional"]),
        "negated-comparison" => (
            Check::NegatedComparison {
                operator: p.text("operator")?,
                except_in: words(&mut p, "except-in"),
                guard: p.flag("except-guard")?,
            },
            &["unary"],
        ),
        "double-negation" => (Check::DoubleNegation, &["unary"]),
        "redundant-cast" => (
            Check::RedundantCast(words(&mut p, "calls")),
            &["unary", "call"],
        ),
        "jump-outside" => {
            let (jump, tags): (&'static str, &'static [&'static str]) =
                match p.text("jump")?.as_str() {
                    "break" => ("break", &["break"]),
                    "continue" => ("continue", &["continue"]),
                    "return" => ("return", &["return"]),
                    other => return Err(format!("param `jump`: unknown jump `{other}`")),
                };
            (
                Check::Jump {
                    jump,
                    inside: words(&mut p, "inside"),
                },
                tags,
            )
        }
        other => return Err(format!("param `check`: unknown check `{other}`")),
    };
    let rule = Simplify {
        meta,
        message: p.text("message")?,
        check,
        tags,
    };
    p.done()?;
    Ok(Box::new(rule))
}

impl Rule for Simplify {
    fn meta(&self) -> &'static RuleMeta {
        self.meta
    }

    fn tags(&self) -> &[&'static str] {
        self.tags
    }

    fn check(&self, node: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        let g = ctx.file.grammar;
        let range = significant_range(node, g);
        let hit = match &self.check {
            Check::CollapsibleIf => collapsible_if(g, node).then(String::new),
            Check::NeedlessBool => needless_bool(g, node).then(String::new),
            Check::SameArms => {
                for later in same_arms(g, node) {
                    ctx.report(
                        significant_range(&later, g),
                        self.message.replace("{name}", ""),
                    );
                }
                return;
            }
            Check::BooleanConditional(a, b) => {
                boolean_conditional(g, node, *a, *b).then(String::new)
            }
            Check::TwistedConditional => twisted(g, node).then(String::new),
            Check::NegatedComparison {
                operator,
                except_in,
                guard,
            } => {
                let Some((raw, inner)) = negated_comparison(g, node, operator, except_in, *guard)
                else {
                    return;
                };
                let edits = negation_edits(ctx.file.text, node, &raw, Some((&inner, operator)));
                self.report(ctx, range, "", edits);
                return;
            }
            Check::DoubleNegation => {
                let Some(raw) = negation(g, node).and_then(|o| negation(g, &un(&o))) else {
                    return;
                };
                let edits = negation_edits(ctx.file.text, node, &raw, None);
                self.report(ctx, range, "", edits);
                return;
            }
            Check::RedundantCast(calls) => redundant_cast(g, node, calls),
            Check::Jump { jump, inside } => jump_outside(g, node, jump, inside).then(String::new),
        };
        if let Some(name) = hit {
            self.report(ctx, range, &name, None);
        }
    }
}

impl Simplify {
    fn report(
        &self,
        ctx: &mut RuleContext<'_>,
        range: TextRange,
        name: &str,
        edits: Option<Vec<Edit>>,
    ) {
        let message = self.message.replace("{name}", name);
        ctx.report_fix(
            range,
            message,
            edits.map(|e| ("Simplify the expression", e)),
        );
    }
}

fn nodes(g: &Grammar, n: &SyntaxNode, label: &str) -> Vec<SyntaxNode> {
    g.slots(n, label).filter_map(|e| e.into_node()).collect()
}

fn node(g: &Grammar, n: &SyntaxNode, label: &str) -> Option<SyntaxNode> {
    g.slot(n, label).and_then(|e| e.into_node())
}

/// The statements of a body: those of a single block, or the statements as they are.
fn unblock(g: &Grammar, mut stmts: Vec<SyntaxNode>) -> Vec<SyntaxNode> {
    if let [only] = &stmts[..]
        && (g.has_tag(only.kind(), "block") || g.has_tag(only.kind(), "statements"))
    {
        stmts = nodes(g, only, "items");
    }
    stmts
}

fn body(g: &Grammar, n: &SyntaxNode) -> Vec<SyntaxNode> {
    unblock(g, nodes(g, n, "then"))
}

fn is_statement_conditional(g: &Grammar, n: &SyntaxNode) -> bool {
    g.has_tag(n.kind(), "conditional") && !g.has_tag(n.kind(), "expression")
}

fn has_condition(g: &Grammar, n: &SyntaxNode) -> bool {
    g.slot(n, "condition").is_some()
}

/// Is the `if` an `elif`: a clause of a chain, or the `if` in the `else` of another?
fn is_elif(g: &Grammar, n: &SyntaxNode) -> bool {
    g.has_tag(n.kind(), "branch")
        || n.parent().is_some_and(|p| {
            is_statement_conditional(g, &p) && node(g, &p, "else").as_ref() == Some(n)
        })
}

/// A branch of the chain after the first: its node, whether it has a test, its statements.
type Branch = (SyntaxNode, bool, Vec<SyntaxNode>);

/// The branches after `n`: its later clauses, or what its `else` holds (an `if` goes on).
fn following(g: &Grammar, n: &SyntaxNode) -> Vec<Branch> {
    let mut out = Vec::new();
    if g.has_tag(n.kind(), "branch") {
        let mut next = n.next_sibling();
        while let Some(s) = next.filter(|s| g.has_tag(s.kind(), "branch")) {
            out.push((s.clone(), has_condition(g, &s), body(g, &s)));
            next = s.next_sibling();
        }
        return out;
    }
    for e in nodes(g, n, "else") {
        if g.has_tag(e.kind(), "branch") {
            out.push((e.clone(), has_condition(g, &e), body(g, &e)));
        } else if is_statement_conditional(g, &e) {
            out.push((e.clone(), true, body(g, &e)));
            out.extend(following(g, &e));
        } else {
            out.push((e.clone(), false, unblock(g, vec![e])));
        }
    }
    out
}

fn has_more(g: &Grammar, n: &SyntaxNode) -> bool {
    !following(g, n).is_empty()
}

fn collapsible_if(g: &Grammar, n: &SyntaxNode) -> bool {
    if !is_statement_conditional(g, n) || !has_condition(g, n) || has_more(g, n) {
        return false;
    }
    matches!(&body(g, n)[..], [s] if is_statement_conditional(g, s)
        && has_condition(g, s) && !has_more(g, s))
}

fn boolean(g: &Grammar, e: &SyntaxNode) -> Option<bool> {
    let e = un(e);
    if !g.has_tag(e.kind(), "literal") {
        return None;
    }
    match code_text(&e).as_str() {
        "true" | "True" => Some(true),
        "false" | "False" => Some(false),
        _ => None,
    }
}

fn bool_return(g: &Grammar, s: &SyntaxNode) -> Option<bool> {
    if !g.has_tag(s.kind(), "return") {
        return None;
    }
    boolean(g, &node(g, s, "value")?)
}

fn only_bool_return(g: &Grammar, body: &[SyntaxNode]) -> Option<bool> {
    match body {
        [s] => bool_return(g, s),
        _ => None,
    }
}

fn needless_bool(g: &Grammar, n: &SyntaxNode) -> bool {
    if !is_statement_conditional(g, n) || !has_condition(g, n) {
        return false;
    }
    let Some(a) = only_bool_return(g, &body(g, n)) else {
        return false;
    };
    let rest = following(g, n);
    let other = match &rest[..] {
        [] if !is_elif(g, n) => n.next_sibling().and_then(|s| bool_return(g, &s)),
        [(_, false, b)] => only_bool_return(g, b),
        _ => None,
    };
    other.is_some_and(|b| b != a)
}

fn key(b: &[SyntaxNode]) -> String {
    b.iter().map(code_text).collect::<Vec<_>>().join("\n")
}

/// The branches (not the first) that repeat the body of the one before them.
fn same_arms(g: &Grammar, n: &SyntaxNode) -> Vec<SyntaxNode> {
    if !is_statement_conditional(g, n) || is_elif(g, n) {
        return Vec::new();
    }
    let mut previous = key(&body(g, n));
    let mut out = Vec::new();
    for (node, tested, stmts) in following(g, n) {
        if !tested {
            break;
        }
        let k = key(&stmts);
        if k == previous && !k.is_empty() {
            out.push(node);
        }
        previous = k;
    }
    out
}

fn boolean_conditional(g: &Grammar, n: &SyntaxNode, then: bool, other: bool) -> bool {
    g.has_tag(n.kind(), "expression")
        && node(g, n, "then").is_some_and(|t| boolean(g, &t) == Some(then))
        && node(g, n, "else").is_some_and(|e| boolean(g, &e) == Some(other))
}

/// The operand of a logical negation (`not x`, `!x`).
fn negation(g: &Grammar, n: &SyntaxNode) -> Option<SyntaxNode> {
    let n = un(n);
    if !g.has_tag(n.kind(), "unary") {
        return None;
    }
    let op = g.slot(&n, "operator")?;
    matches!(op.as_token()?.text(), "not" | "!").then_some(())?;
    node(g, &n, "operand")
}

fn twisted(g: &Grammar, n: &SyntaxNode) -> bool {
    let (Some(t), Some(c), Some(e)) = (
        node(g, n, "then"),
        node(g, n, "condition"),
        node(g, n, "else"),
    ) else {
        return false;
    };
    let (e, tag) = (un(&e), "reference");
    g.has_tag(n.kind(), "expression")
        && boolean(g, &t).is_none()
        && boolean(g, &e).is_none()
        && negation(g, &c).map(|o| un(&o)).is_some_and(|o| {
            g.has_tag(o.kind(), tag) && g.has_tag(e.kind(), tag) && code_text(&o) == code_text(&e)
        })
}

fn function_name(g: &Grammar, n: &SyntaxNode) -> String {
    n.ancestors()
        .skip(1)
        .find(|a| g.has_tag(a.kind(), "function"))
        .and_then(|f| node(g, &f, "name"))
        .map(|name| code_text(&name))
        .unwrap_or_default()
}

/// Is the expression in the test of an `if` whose only statement throws (an argument check)?
fn guard(g: &Grammar, n: &SyntaxNode) -> bool {
    let mut child = n.clone();
    for a in n.ancestors().skip(1) {
        if is_statement_conditional(g, &a)
            && node(g, &a, "condition").as_ref() == Some(&outer(&child))
        {
            return matches!(&body(g, &a)[..], [s] if g.has_tag(s.kind(), "throw"));
        }
        if g.has_tag(a.kind(), "statement") {
            return false;
        }
        child = a;
    }
    false
}

/// The comparison `not` negates, when it is a single `operator` comparison.
fn negated_comparison(
    g: &Grammar,
    n: &SyntaxNode,
    operator: &str,
    except_in: &[String],
    except_guard: bool,
) -> Option<(SyntaxNode, SyntaxNode)> {
    let raw = negation(g, n)?;
    let operand = un(&raw);
    if !g.has_tag(operand.kind(), "binary") {
        return None;
    }
    // a chain (`a == b == c`) has an operator after its right operand too
    let mut ops = operand
        .children_with_tokens()
        .filter_map(|e| e.into_token())
        .filter(|t| g.has_tag(t.kind(), "operator"));
    if ops.next().is_none_or(|t| t.text() != operator) || ops.next().is_some() {
        return None;
    }
    if (!except_in.is_empty() && except_in.contains(&function_name(g, n)))
        || (except_guard && guard(g, n))
    {
        return None;
    }
    Some((raw, operand))
}

/// The edits of a fix that drops the negation around `raw` (the operand as written) and, with
/// `swap` (the comparison and its operator), turns the operator into its opposite; none when
/// something but negations and balanced brackets is around the operand.
fn negation_edits(
    text: &str,
    n: &SyntaxNode,
    raw: &SyntaxNode,
    swap: Option<(&SyntaxNode, &str)>,
) -> Option<Vec<Edit>> {
    let (whole, inside) = (n.text_range(), raw.text_range());
    let head = TextRange::new(whole.start(), inside.start());
    let tail = TextRange::new(inside.end(), whole.end());
    let only = |r: TextRange, extra: char| {
        text[r]
            .chars()
            .all(|c| c.is_whitespace() || c == extra || c == '!' || c.is_alphabetic())
    };
    let count = |r: TextRange, c: char| text[r].matches(c).count();
    if !only(head, '(') || !only(tail, ')') || count(head, '(') != count(tail, ')') {
        return None;
    }
    let mut edits = vec![Edit::delete(head), Edit::delete(tail)];
    if let Some((comparison, op)) = swap {
        let token = comparison
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.text() == op)?;
        edits.push(Edit::replace(
            token.text_range(),
            if op == "==" { "!=" } else { "==" },
        ));
    }
    Some(edits)
}

fn in_boolean_context(g: &Grammar, n: &SyntaxNode, calls: &[String]) -> bool {
    let outer = outer(n);
    let Some(parent) = outer.parent() else {
        return false;
    };
    let kind = parent.kind();
    if g.has_tag(kind, "conditional") || g.has_tag(kind, "loop") {
        return node(g, &parent, "condition").as_ref() == Some(&outer);
    }
    if g.has_tag(kind, "unary") {
        return g
            .slot(&parent, "operator")
            .and_then(|o| o.as_token().map(|t| t.text() == "!"))
            == Some(true);
    }
    g.has_tag(kind, "call")
        && is_named(g, &parent, calls)
        && node(g, &parent, "callee").as_ref() != Some(&outer)
}

fn is_named(g: &Grammar, call: &SyntaxNode, names: &[String]) -> bool {
    node(g, call, "callee")
        .is_some_and(|c| u32::from(c.text_range().len()) <= 24 && names.contains(&code_text(&c)))
}

fn redundant_cast(g: &Grammar, n: &SyntaxNode, calls: &[String]) -> Option<String> {
    let found = if g.has_tag(n.kind(), "unary") {
        negation(g, n)
            .and_then(|o| negation(g, &un(&o)))
            .map(|_| "double negation".to_string())
    } else {
        is_named(g, n, calls).then(|| {
            format!(
                "{} call",
                node(g, n, "callee")
                    .map(|c| code_text(&c))
                    .unwrap_or_default()
            )
        })
    }?;
    in_boolean_context(g, n, calls).then_some(found)
}

fn jump_outside(g: &Grammar, n: &SyntaxNode, jump: &str, inside: &[String]) -> bool {
    if g.slot(n, "label").is_some() {
        return false;
    }
    let mut child = n.clone();
    for a in n.ancestors().skip(1) {
        let k = a.kind();
        if jump != "return" {
            let allowed = g.has_tag(k, "loop") && nodes(g, &a, "body").contains(&child)
                || (jump == "break" && inside.iter().any(|t| g.has_tag(k, t)));
            if allowed {
                return false;
            }
        }
        if g.has_tag(k, "function") {
            return jump != "return";
        }
        if g.has_tag(k, "type-declaration") {
            return true;
        }
        child = a;
    }
    true
}
