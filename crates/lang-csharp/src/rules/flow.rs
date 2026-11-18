//! Rules over the flow layer: unused variables, dead stores.
//!
//! concept: unused-variable, new unused-assignment
//! what: A value is stored in a local variable and no path reads it before it is overwritten or the variable goes out of scope (a dead store)

use super::{kind, range_of};
use core_flow::{LocalKind, OpFlags, OpKind, dead_stores};
use core_rules::rule;
use core_rules::{BodyContext, Edit, RuleContext};
use core_syntax::TextRange;

rule! {
    pub UnusedVariable {
        id: "unused-variable",
        category: Cleanup,
        default: Off,
        fix: Safe,
        description: "A local variable or catch variable that is never used (the compiler's CS0168), \
                  or is only assigned constants and never read (CS0219). Variables captured by \
                  lambdas or local functions count as used. Off by default: the compiler already warns (CS0168, CS0219).",
        example: "int total = 0;   // never read\ntry { Run(); } catch (Exception e) { }   // e is never used",
        requires: [Flow],
    }

    fn check_body(&self, body: &BodyContext<'_>, ctx: &mut RuleContext<'_>) {
        let Some(flow) = &body.flow else { return };
        let graph = flow.graph;
        for (index, local) in graph.locals.iter().enumerate() {
            if !matches!(local.kind(), LocalKind::Declaration | LocalKind::Catch)
                || !local.tracked()
            {
                continue;
            }
            let (mut reads, mut writes, mut non_constant) = (0, 0, 0);
            for op in graph.blocks.iter().flat_map(|b| &b.ops) {
                if op.local() != Some(index) {
                    continue;
                }
                match op.kind() {
                    OpKind::Read | OpKind::ReadWrite => reads += 1,
                    OpKind::Write => {
                        writes += 1;
                        non_constant += i32::from(!op.has(OpFlags::CONSTANT));
                    }
                    _ => {}
                }
            }
            let message = match (reads, writes, non_constant) {
                (0, 0, _) => format!("variable `{}` is declared but never used", local.name),
                (0, _, 0) => format!(
                    "variable `{}` is assigned but its value is never used",
                    local.name
                ),
                _ => continue,
            };
            let range = range_of(local.start, local.len);
            let fix = removal(body, local.kind(), range)
                .map(|edit| (format!("Remove `{}`", local.name), vec![edit]));
            ctx.report_fix(range, message, fix);
        }
    }
}

/// The edit that removes the variable: its whole declaration statement when it declares nothing
/// else, or a catch variable's name.
fn removal(body: &BodyContext<'_>, local: LocalKind, name: TextRange) -> Option<Edit> {
    let file = body.file;
    let at = body.node.covering_element(name);
    let node = at.as_node().cloned().or_else(|| at.as_token()?.parent())?;
    if local == LocalKind::Catch {
        let text = &file.text[..usize::from(name.start())];
        let start = text.trim_end().len();
        return Some(Edit::delete(range_of(
            start as u32,
            u32::from(name.end()) - start as u32,
        )));
    }
    let statement = node
        .ancestors()
        .find(|n| n.kind() == kind::LOCAL_DECLARATION_STATEMENT)?;
    let declarators = statement
        .descendants()
        .filter(|n| n.kind() == kind::VARIABLE_DECLARATOR)
        .count();
    (declarators == 1).then(|| {
        Edit::delete(core_rules::tree::significant_range(
            &statement,
            file.grammar,
        ))
    })
}

rule! {
    pub UnusedAssignment {
        id: "unused-assignment",
        category: Cleanup,
        default: Off,
        description: "A value is stored in a local variable and no path reads it before it is \
                  overwritten or the variable goes out of scope (a dead store). Variables that \
                  are captured or whose address is taken are not checked; a variable that is \
                  never read at all is reported by `unused-variable` instead. Overlaps the SDK analyzer IDE0059 (unnecessary assignment).",
        example: "int n = 0;\nn = Count();   // the 0 is never read",
        requires: [Flow],
    }

    fn check_body(&self, body: &BodyContext<'_>, ctx: &mut RuleContext<'_>) {
        let Some(flow) = &body.flow else { return };
        let graph = flow.graph;
        let reach = flow.reachability();
        for store in dead_stores(graph, &reach) {
            let op = &graph.blocks[store.block].ops[store.op];
            let Some(index) = op.local() else { continue };
            let local = &graph.locals[index];
            let ever_read = graph.blocks.iter().flat_map(|b| &b.ops).any(|o| {
                o.local() == Some(index) && matches!(o.kind(), OpKind::Read | OpKind::ReadWrite)
            });
            if ever_read {
                ctx.report(
                    range_of(op.start, op.len),
                    format!("the value assigned to `{}` is never read", local.name),
                );
            }
        }
    }
}
