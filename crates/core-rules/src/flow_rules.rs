//! `unreachable-code` and `no-fallthrough`: the flow family. Both read the graph that
//! `core_flow::build` makes from the statement roles of the grammar (`#block`, `#loop`, `#switch`,
//! `#case`, `#try`, `#break`, ...); a grammar tag says what data cannot: `#inert` (a statement that
//! is never reported as unreachable: empty, hoisted, a guard the compiler leaves alone) and
//! `#fallthrough` (a `switch` whose arms run into the next one); the language's `rule` row
//! `case_label_calls "true"` leaves a call statement after a jump in a case body alone (a macro that
//! expands to a label). A provider whose semantic model
//! has flow graphs (C#) gets constant conditions and calls that never return on top.
//!
//! concept: unreachable-code, no-fallthrough
//! what: Flow family: code no path reaches and switch cases that fall through, on the flow graph.
//! rules: unreachable-code, no-fallthrough
//! reads: rule:unreachable-code

use crate::diagnostic::Edit;
use crate::rule::{BodyContext, FileContext, RuleContext};
use crate::tree::{code_start, significant_range};
use crate::{Check, Opt, OptionDecl, rule};
use core_flow::{BodyGraph, build, cases, endless_loop, is_leaf};
use core_syntax::{Grammar, SyntaxNode, SyntaxToken, TextRange};

/// Does `n` have a graph of its own (a function, or a kind the provider calls a body)?
fn is_unit(n: &SyntaxNode, file: &FileContext<'_>) -> bool {
    file.grammar.has_tag(n.kind(), "function") || file.body_kinds.contains(&n.kind())
}

/// The statements that none of whose operations can run: the starts of those of `graph` that
/// `live` says nothing of them runs, minus the ones around code that does.
fn dead_starts(graph: &BodyGraph, live: impl Fn(usize, usize) -> bool) -> Vec<u32> {
    let (mut dead, mut alive) = (Vec::new(), Vec::new());
    for (b, block) in graph.blocks.iter().enumerate() {
        for (i, op) in block.ops.iter().enumerate().filter(|(_, o)| o.stmt_len > 0) {
            let set = if live(b, i) { &mut alive } else { &mut dead };
            set.push((op.stmt_start, op.stmt_len));
        }
    }
    dead.retain(|&(s, l)| !alive.iter().any(|&(a, al)| a >= s && a + al <= s + l));
    let mut starts: Vec<u32> = dead.into_iter().map(|d| d.0).collect();
    starts.sort_unstable();
    starts
}

/// Goes down only into the nodes that hold a dead start (`dead` is sorted).
fn walk(n: &SyntaxNode, ctx: &mut RuleContext<'_>, dead: &[u32]) {
    let file = ctx.file;
    let g = file.grammar;
    let start = |c: &SyntaxNode| u32::from(code_start(c, g));
    let is_dead = |s: u32| dead.binary_search(&s).is_ok();
    let mut reported = false;
    // a call statement in a case body may be a macro that expands to a label: the code after it runs
    let labels = case_label_calls(g) && g.has_tag(n.kind(), "case");
    let mut label = false;
    for child in n.children() {
        let range = child.text_range();
        let at = dead.partition_point(|&s| s < u32::from(range.start()));
        if dead.get(at).is_none_or(|&s| s > u32::from(range.end())) || is_unit(&child, file) {
            continue;
        }
        // a handler that no throw can reach
        if g.has_tag(child.kind(), "catch")
            && is_dead(start(&child))
            && let Some(first) = g.slots(&child, "body").find_map(|e| e.into_node())
        {
            let last = g.slots(&child, "body").filter_map(|e| e.into_node()).last();
            let end = last.map_or(first.text_range(), |l| significant_range(&l, g));
            let range = TextRange::new(significant_range(&first, g).start(), end.end());
            ctx.report(range, "unreachable code");
            continue;
        }
        if is_dead(start(&child)) {
            label |= labels && is_call_statement(&child, g);
            if label {
                reported = false;
                continue;
            }
            // a run of unreachable statements is one report; a statement that is not reported ends it
            let target = !inert(&child, g);
            if target && !reported {
                let range = significant_range(&child, g);
                let fix = (
                    "Remove the unreachable statement",
                    vec![Edit::delete(range)],
                );
                ctx.report_fix(range, "unreachable code", fix);
            }
            reported = target;
            continue;
        }
        walk(&child, ctx, dead);
    }
}

/// Does the language's `rule` row read a call statement in a case body as a possible label?
fn case_label_calls(g: &Grammar) -> bool {
    g.rule_row("unreachable-code").is_some_and(|r| {
        r.options
            .iter()
            .any(|(n, v)| n == "case_label_calls" && v.first().is_some_and(|v| v == "true"))
    })
}

/// Is `s` a statement that is one call (`name(args);`)?
fn is_call_statement(s: &SyntaxNode, g: &Grammar) -> bool {
    g.has_tag(s.kind(), "expression-statement")
        && g.slot(s, "expression")
            .and_then(|e| e.into_node())
            .is_some_and(|e| g.has_tag(e.kind(), "call"))
}

/// Is the statement left alone: `#inert`, or `#inert-bare` with a `hoisted` keyword and no declarator
/// with a value (`var x;` hoists, so code after a `return` may use it; `let x;` does not)?
fn inert(s: &SyntaxNode, g: &Grammar) -> bool {
    g.has_tag(s.kind(), "inert")
        || (g.has_tag(s.kind(), "inert-bare")
            && s.children_with_tokens()
                .any(|e| e.as_token().is_some_and(|t| g.is_hoisted(t.text())))
            && !s.children().any(|d| g.slot(&d, "init").is_some()))
}

/// Where a unit of flow ends when going up from `n`: its function (or the file root), and whether
/// something that surely runs after `n` follows it: a statement after `n` or after one of the
/// statements around it, up to the first one that code can leave by another way (a loop that may
/// not run, a handler, an `if` without `else`). That is what a jump needs to leave code behind it.
/// `None` for the unit of a body kind when the provider's flow layer is there to read it.
fn unit_of(n: &SyntaxNode, file: &FileContext<'_>) -> Option<(SyntaxNode, bool)> {
    let g = file.grammar;
    // a handler is reported without a statement after it: it needs its unit anyway
    let handler = g.has_tag(n.kind(), "catch");
    let (mut followed, mut open) = (false, true);
    let mut at = n.clone();
    while let Some(parent) = at.parent() {
        let k = parent.kind();
        let body = file.body_kinds.contains(&k);
        if body && file.semantics.is_some() {
            return None;
        }
        let function = body || g.has_tag(k, "function");
        // only the siblings in a list of statements are statements that follow
        let list = function
            || ["block", "statements", "case"]
                .iter()
                .any(|t| g.has_tag(k, t))
            || parent.parent().is_none();
        followed |= open && list && at.next_sibling().is_some();
        if function {
            return Some((parent, followed));
        }
        // nothing is left to learn once a statement is known to follow
        if open && !followed {
            open = !(g.has_tag(k, "loop")
                || g.has_tag(k, "catch")
                || (g.has_tag(k, "conditional") && g.slot(&parent, "else").is_none())
                || (is_leaf(g, &parent) && !switch_part(g, &parent)));
            if !open && !handler {
                return None;
            }
        }
        at = parent;
    }
    Some((at, followed))
}

/// A `case`, or the wrapper that holds the cases of a `switch` (`cases`): the graph reads below it.
fn switch_part(g: &Grammar, n: &SyntaxNode) -> bool {
    g.has_tag(n.kind(), "case") || n.parent().is_some_and(|p| g.has_tag(p.kind(), "switch"))
}

/// One unit of control flow: from the roles of its statements.
fn from_roles(n: &SyntaxNode, ctx: &mut RuleContext<'_>) {
    let file = ctx.file;
    // one block: straight-line code, nothing in it can be unreachable
    if let Some(flow) = build(file.grammar, file.text, n)
        && flow.graph.blocks.len() > 1
    {
        walk(n, ctx, flow.reach().dead_starts());
    }
}

rule! {
    pub UnreachableCode {
        id: "unreachable-code",
        category: Suspicious,
        default: Warn,
        fix: Suggestion,
        description: "A statement that can never run: after `return`, `throw`, `break` or `continue`, \
                  after an `if`/`else` or `try` that always jumps, or after a loop that never ends. \
                  Only the first statement of a run is reported. Where the language has a flow layer \
                  (C#) it also covers constant conditions and calls that do not return \
                  (`does_not_return` in `semantics:`). Hoisted declarations and empty statements are \
                  left out. Removing the statement is offered as an editor quick fix only, because \
                  unreachable code usually signals a bug.",
        example: "function f() { return; a(); }   // a() is reported",
        uses: [Flow],
        options: [OptionDecl {
            name: "non_returning_calls",
            default: Opt::Bool(true),
            check: Check::Any,
            description: "With a flow layer, treat the code after a call that never returns \
                      (`[DoesNotReturn]`, `does_not_return` in `semantics:`) as unreachable. A \
                      compiler does not, so turn this off to report exactly what it reports.",
        }],
        tags: &["return", "throw", "break", "continue", "loop", "catch"],
    }

    // Only a jump, a loop that never ends or a handler can leave code unreachable: the unit
    // around one is built once, when the first of them is met with something after it.
    fn check(&self, node: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        let (file, g) = (ctx.file, ctx.file.grammar);
        if g.has_tag(node.kind(), "loop") && !endless_loop(g, file.text, node) {
            return;
        }
        if let Some((unit, followed)) = unit_of(node, file)
            && (followed || g.has_tag(node.kind(), "catch"))
            && ctx.first_visit(unit.text_range())
        {
            from_roles(&unit, ctx);
        }
    }

    fn check_body(&self, body: &BodyContext<'_>, ctx: &mut RuleContext<'_>) {
        let Some(flow) = &body.flow else {
            return;
        };
        let reach = if ctx.options.bool("non_returning_calls") {
            flow.reachability()
        } else {
            flow.compiler_reachability()
        };
        let dead = dead_starts(flow.graph, |b, i| reach.op_reachable(b, i));
        walk(body.node, ctx, &dead);
    }
}

/// The comment closest before `t` (only trivia between).
fn comment_before(t: &SyntaxToken, g: &Grammar) -> Option<String> {
    std::iter::successors(t.prev_token(), |p| p.prev_token())
        .take_while(|p| g.is_trivia(p.kind()))
        .find(|p| g.has_tag(p.kind(), "comment"))
        .map(|p| p.text().to_string())
}

/// Does the comment say the fallthrough is meant (`falls through`, `fall-through`, `fallthru`)?
/// A suppression comment that only names the rule does not.
fn says_fallthrough(comment: &str) -> bool {
    let text = comment.to_ascii_lowercase();
    let words = text.trim_start_matches(['/', '*', '#', ' ']);
    let squeezed: String = text.chars().filter(|c| !matches!(c, ' ' | '-')).collect();
    !(words.starts_with("eslint") || words.starts_with("sudsy"))
        && ["fallthrough", "fallsthrough", "fallthru", "fallsthru"]
            .iter()
            .any(|p| squeezed.contains(p))
}

fn first_token(n: &SyntaxNode, g: &Grammar) -> Option<SyntaxToken> {
    n.token_at_offset(code_start(n, g)).right_biased()
}

rule! {
    pub NoFallthrough {
        id: "no-fallthrough",
        category: Suspicious,
        default: Warn,
        description: "A `case` with statements that can reach the next `case` or `default`, or an \
                  empty one followed by a blank line, without a comment that says `falls through` \
                  before the next one (or at the end of its block).",
        example: "switch (a) { case 1: b(); case 2: c(); }   // the second case is reported",
        tags: &["fallthrough"],
    }

    fn check(&self, node: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        let file = ctx.file;
        let g = file.grammar;
        let Some(flow) = build(g, file.text, node) else {
            return;
        };
        let reached = flow.reach();
        let all = cases(g, node);
        for pair in all.windows(2) {
            let (case, next) = (&pair[0], &pair[1]);
            let body: Vec<_> = g.slots(case, "body").filter_map(|e| e.into_node()).collect();
            let Some(next_token) = first_token(next, g) else {
                continue;
            };
            let gap = TextRange::new(significant_range(case, g).end(), next_token.text_range().start());
            let blank = file.text[gap].matches('\n').count() >= 2;
            let at: u32 = code_start(case, g).into();
            let alive = flow
                .case_ends
                .iter()
                .any(|&(s, end)| s == at && reached.block[end]);
            if !alive || (body.is_empty() && !blank) {
                continue;
            }
            let said = |c: Option<String>| c.is_some_and(|m| says_fallthrough(&m));
            let in_block = match body.as_slice() {
                [b] if g.has_tag(b.kind(), "block") => {
                    said(b.last_token().and_then(|close| comment_before(&close, g)))
                }
                _ => false,
            };
            if !in_block && !said(comment_before(&next_token, g)) {
                let name = next_token.text();
                ctx.report(
                    significant_range(next, g),
                    format!("Expected a `break` statement before `{name}`."),
                );
            }
        }
    }
}

/// Is any switch kind of `g` a fall-through one? Then `no-fallthrough` is listed.
pub fn has_fallthrough(g: &Grammar) -> bool {
    !g.nodes_with_tag("fallthrough").is_empty()
}

/// Does `g` have the statement roles `unreachable-code` reads?
pub fn has_jumps(g: &Grammar) -> bool {
    ["return", "break", "block"]
        .iter()
        .all(|t| !g.nodes_with_tag(t).is_empty())
}
