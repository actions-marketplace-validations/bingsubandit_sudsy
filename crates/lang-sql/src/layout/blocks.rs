//! The indentation of procedural blocks. The MySQL parse leaves `BEGIN ... END`, `LOOP`,
//! `WHILE` and `REPEAT` as flat statements with no indent, and its T-SQL dialect puts no indent
//! around the condition and the body of `IF`, `ELSE` and `WHILE`, so the indents sqlfluff's dialects have are put
//! on the points here, from the keywords and statements.

use super::seq::{NONE, Seq};
use crate::generated::kind as k;

fn word(seq: &Seq, i: usize) -> String {
    seq.blocks
        .get(i)
        .map_or(String::new(), |b| b.text.to_ascii_uppercase())
}

/// Is block `i` the first word of a statement (after `;`, a label, or an opening keyword)?
fn starts_statement(seq: &Seq, i: usize) -> bool {
    i == 0
        || matches!(seq.blocks[i - 1].kind, k::SEMICOLON | k::COLON)
        || matches!(
            word(seq, i - 1).as_str(),
            "BEGIN" | "LOOP" | "THEN" | "ELSE" | "DO" | "REPEAT"
        )
}

fn indent(seq: &mut Seq, at: usize) {
    if at < seq.points.len() {
        seq.points[at].metas.push(1);
    }
}

fn dedent(seq: &mut Seq, at: usize) {
    seq.points[at].metas.push(-1);
}

pub fn add_indents(seq: &mut Seq) {
    let (mut depth, mut cases) = (0usize, 0usize);
    let (mut do_pending, mut after_until) = (false, false);
    for i in 0..seq.blocks.len() {
        if seq.blocks[i].kind != k::KEYWORD {
            continue;
        }
        let stmt = starts_statement(seq, i);
        let next = word(seq, i + 1);
        match word(seq, i).as_str() {
            "BEGIN" if !matches!(next.as_str(), "WORK" | "TRANSACTION" | "TRAN" | ";" | "") => {
                depth += 1;
                indent(seq, i + 1);
            }
            "LOOP" if word(seq, i.wrapping_sub(1)) != "END" => {
                depth += 1;
                indent(seq, i + 1);
            }
            "REPEAT" if stmt => {
                depth += 1;
                indent(seq, i + 1);
            }
            "WHILE" if stmt && word(seq, i.wrapping_sub(1)) != "END" => do_pending = true,
            "CASE" if stmt => cases += 1,
            "DO" if do_pending => {
                do_pending = false;
                depth += 1;
                indent(seq, i + 1);
            }
            "UNTIL" if stmt && depth > 0 => {
                dedent(seq, i);
                depth -= 1;
                after_until = true;
            }
            "END" if stmt => match next.as_str() {
                "CASE" => cases = cases.saturating_sub(1),
                "REPEAT" if after_until => after_until = false,
                _ if depth > 0 => {
                    dedent(seq, i);
                    depth -= 1;
                }
                _ => {}
            },
            _ => {}
        }
    }
}

/// T-SQL: the condition of `IF` and `WHILE`, and the statement after it or after `ELSE`, are indented.
pub fn add_tsql_indents(seq: &mut Seq) {
    for id in 0..seq.nodes.len() as u32 {
        let node = &seq.nodes[id as usize];
        if node.kind != k::STATEMENT || node.first == NONE {
            continue;
        }
        let head = word(seq, node.first as usize);
        if head != "IF" && head != "WHILE" {
            continue;
        }
        let mut after_condition = false;
        let mut bodies: Vec<(usize, usize)> = Vec::new();
        for c in seq.children(id).collect::<Vec<_>>() {
            let (kind, first, last) = {
                let child = &seq.nodes[c as usize];
                (child.kind, child.first as usize, child.last as usize)
            };
            if first == NONE as usize {
                continue;
            }
            if kind == k::EXPRESSION && !after_condition {
                indent(seq, first);
                seq.points[first].implicit = true;
                dedent(seq, last + 1);
                after_condition = true;
            } else if kind == k::STATEMENT && (after_condition || word(seq, first - 1) == "ELSE") {
                bodies.push((first, last));
                after_condition = false;
            }
        }
        for (first, last) in bodies {
            indent(seq, first);
            dedent(seq, last + 1);
        }
    }
}
