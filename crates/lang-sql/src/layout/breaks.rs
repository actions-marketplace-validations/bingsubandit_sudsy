//! Where line breaks go: operators and commas next to them (sqlfluff's LT03, LT04), set
//! operators (LT11), clause keywords (LT14), select targets (LT09), CTE brackets and blank lines
//! (LT07, LT08), blank lines and the start of the file (LT13, LT15).
//!
//! concept: new clause-line-breaks
//! what: Where line breaks go: operators, commas, set operators, clause keywords, select targets.
//! tools: sqlfluff LT03 LT04 LT09 LT11 LT14

use super::seq::{NONE, Seq};
use super::space::respace_point;
use crate::generated::kind as k;
use std::borrow::Cow;

/// The indentation of the line that holds block `i`.
fn line_indent(seq: &Seq, i: usize) -> String {
    let at = (0..=i).rev().find(|&p| seq.points[p].newlines > 0);
    seq.points[at.unwrap_or(0)].ws.to_string()
}

/// A line break before block `i` with `indent`.
fn break_before(seq: &mut Seq, i: usize, indent: &str) {
    let p = &mut seq.points[i];
    p.newlines = p.newlines.max(1);
    p.ws = Cow::Owned(indent.to_string());
}

#[derive(Clone, Copy, PartialEq)]
pub enum Pos {
    Leading,
    Trailing,
    Alone,
}

pub struct Span {
    pub s: usize,
    pub e: usize,
    pub pos: Pos,
    strict: bool,
}

/// The points around a span: the adjacent one, the nearest one with a line break (comments are
/// skipped) and the nearest one with code behind it.
struct Side {
    adj: usize,
    newline: usize,
    code: usize,
}

fn prev_side(seq: &Seq, s: usize) -> Side {
    let mut p = s;
    while p > 1 && seq.points[p].newlines == 0 && seq.is_comment(p - 1) {
        p -= 1;
    }
    let newline = p;
    while p > 1 && seq.is_comment(p - 1) {
        p -= 1;
    }
    Side {
        adj: s,
        newline,
        code: p,
    }
}

fn next_side(seq: &Seq, e: usize) -> Side {
    let last = seq.blocks.len() - 1;
    let mut p = e + 1;
    while p < last && seq.points[p].newlines == 0 && seq.is_comment(p) {
        p += 1;
    }
    let newline = p;
    while p < last && seq.is_comment(p) {
        p += 1;
    }
    Side {
        adj: e + 1,
        newline,
        code: p,
    }
}

pub fn spans(seq: &Seq, comma: Pos) -> Vec<Span> {
    let mut out = Vec::new();
    for i in 1..seq.blocks.len() - 1 {
        let b = &seq.blocks[i];
        let node = &seq.nodes[b.node as usize];
        let (pos, strict, s, e) = match (b.kind, node.kind) {
            (k::COMMA, _) => (comma, false, i, i),
            (k::BINARY_OPERATOR | k::BINARY_KEYWORD, _) => (Pos::Leading, false, i, i),
            (_, k::COMPARISON_OPERATOR) if node.first == i as u32 => {
                (Pos::Leading, false, i, node.last as usize)
            }
            (_, k::SET_OPERATOR) if node.first == i as u32 => {
                (Pos::Alone, true, i, node.last as usize)
            }
            _ => continue,
        };
        out.push(Span { s, e, pos, strict });
    }
    out
}

/// Keeps operators, commas and set operators where they belong around line breaks.
pub fn rebreak(seq: &mut Seq, comma: Pos) {
    for sp in spans(seq, comma) {
        let (prev, next) = (prev_side(seq, sp.s), next_side(seq, sp.e));
        let before = seq.points[prev.newline].newlines;
        let after = seq.points[next.newline].newlines;
        let both = before > 0 && after > 0;
        if both || (before + after == 0 && !sp.strict) {
            continue;
        }
        match sp.pos {
            Pos::Leading if before == 0 && next.adj == next.code && after == 1 => {
                let indent = seq.points[next.adj].ws.to_string();
                break_before(seq, prev.adj, &indent);
                respace_point(seq, next.adj, true);
            }
            Pos::Trailing if after == 0 && prev.adj == prev.code && before == 1 => {
                let indent = seq.points[prev.adj].ws.to_string();
                break_before(seq, next.adj, &indent);
                respace_point(seq, prev.adj, true);
            }
            Pos::Alone => {
                if after == 0 {
                    let indent = line_indent(seq, sp.e);
                    break_before(seq, next.adj, &indent);
                }
                if seq.points[prev.adj].newlines == 0 {
                    let indent = line_indent(seq, sp.s);
                    break_before(seq, prev.adj, &indent);
                }
            }
            _ => {}
        }
    }
}

/// Clause keywords start a line (LT14): WHERE, GROUP BY, HAVING and ORDER BY, but not an ORDER BY
/// inside a window.
pub fn keywords(seq: &mut Seq) {
    for i in 1..seq.blocks.len() - 1 {
        let b = &seq.blocks[i];
        let node = &seq.nodes[b.node as usize];
        let clause = matches!(
            node.kind,
            k::WHERE_CLAUSE | k::GROUPBY_CLAUSE | k::HAVING_CLAUSE | k::ORDERBY_CLAUSE
        );
        if b.kind != k::KEYWORD || !clause || node.first != i as u32 {
            continue;
        }
        let mut n = node.parent;
        let mut excluded = false;
        while n != NONE {
            excluded |= seq.nodes[n as usize].kind == k::WINDOW_SPECIFICATION;
            n = seq.nodes[n as usize].parent;
        }
        if excluded {
            continue;
        }
        // The span ends before the first indent after the keyword.
        let mut e = i;
        while e < node.last as usize && !seq.points[e + 1].metas.contains(&1) {
            e += 1;
        }
        let (prev, next) = (prev_side(seq, i), next_side(seq, e));
        let before = seq.points[prev.newline].newlines;
        if before > 0 {
            continue;
        }
        let indent = if seq.points[next.adj].newlines > 0 {
            seq.points[next.adj].ws.to_string()
        } else {
            String::new()
        };
        break_before(seq, prev.adj, &indent);
        respace_point(seq, next.adj, true);
    }
}

/// The width of blocks `a..=b` as laid out on one line.
fn span_len(seq: &Seq, a: usize, b: usize) -> usize {
    (a..=b)
        .map(|i| {
            seq.blocks[i].text.chars().count()
                + if i > a {
                    seq.points[i].ws.chars().count()
                } else {
                    0
                }
        })
        .sum()
}

/// The width of what follows block `b` on its line.
fn tail_len(seq: &Seq, b: usize) -> usize {
    (b + 1..seq.blocks.len())
        .take_while(|&i| seq.points[i].newlines == 0)
        .map(|i| seq.points[i].ws.chars().count() + seq.blocks[i].text.chars().count())
        .sum()
}

/// Select targets, one per line unless there is just one (LT09).
pub fn select_targets(seq: &mut Seq, max_width: usize) {
    for id in 0..seq.nodes.len() as u32 {
        if seq.nodes[id as usize].kind != k::SELECT_CLAUSE || seq.nodes[id as usize].first == NONE {
            continue;
        }
        let targets: Vec<u32> = seq
            .children(id)
            .filter(|&c| seq.nodes[c as usize].kind == k::SELECT_CLAUSE_ELEMENT)
            .collect();
        let clause = &seq.nodes[id as usize];
        let (first, last) = (clause.first as usize, clause.last as usize);
        match targets.len() {
            0 => {}
            1 => {
                let t = targets[0];
                let f = seq.nodes[t as usize].first as usize;
                let broken = (first + 1..=f).any(|p| seq.points[p].newlines > 0);
                let comment = (first + 1..f).any(|b| seq.is_comment(b));
                if !broken || comment || seq.spans_lines(t) {
                    continue;
                }
                let width = line_indent(seq, first).chars().count()
                    + (first..f)
                        .map(|b| seq.blocks[b].text.chars().count() + 1)
                        .sum::<usize>()
                    + span_len(seq, f, seq.nodes[t as usize].last as usize)
                    + tail_len(seq, seq.nodes[t as usize].last as usize);
                if max_width > 0 && width > max_width {
                    continue;
                }
                for p in first + 1..=f {
                    seq.points[p].newlines = 0;
                    seq.points[p].set_ws(" ");
                }
                let parent = seq.nodes[id as usize].parent;
                if parent != NONE && seq.nodes[parent as usize].kind == k::SELECT_STATEMENT {
                    let end = seq.nodes[parent as usize].last as usize;
                    if end > last {
                        break_before(seq, last + 1, "");
                    }
                }
            }
            _ => {
                let mut prev = (first..seq.nodes[targets[0] as usize].first as usize)
                    .rev()
                    .find(|&b| !seq.is_comment(b) && seq.blocks[b].kind != k::COMMA)
                    .unwrap_or(first);
                for &t in &targets {
                    let f = seq.nodes[t as usize].first as usize;
                    if (prev + 1..=f).all(|p| seq.points[p].newlines == 0) {
                        break_before(seq, f, "");
                    }
                    prev = seq.nodes[t as usize].last as usize;
                }
                let parent = seq.nodes[id as usize].parent;
                let from = (parent != NONE)
                    .then(|| {
                        seq.children(parent)
                            .find(|&c| seq.nodes[c as usize].kind == k::FROM_CLAUSE)
                    })
                    .flatten();
                if let Some(from) = from {
                    let (a, b) = (
                        seq.nodes[targets[targets.len() - 1] as usize].first as usize,
                        seq.nodes[from as usize].first as usize,
                    );
                    if b > a && (a + 1..=b).all(|p| seq.points[p].newlines == 0) {
                        break_before(seq, b, "");
                    }
                }
            }
        }
    }
}

/// The closing bracket of a CTE that spans lines starts a line (LT07), and a blank line follows
/// each CTE (LT08).
pub fn ctes(seq: &mut Seq) {
    for id in 0..seq.nodes.len() as u32 {
        if seq.nodes[id as usize].kind != k::WITH_COMPOUND_STATEMENT {
            continue;
        }
        let ctes: Vec<u32> = seq
            .children(id)
            .filter(|&c| seq.nodes[c as usize].kind == k::COMMON_TABLE_EXPRESSION)
            .collect();
        for cte in ctes {
            let Some(bracket) = seq
                .children(cte)
                .filter(|&c| seq.nodes[c as usize].kind == k::BRACKETED)
                .last()
            else {
                continue;
            };
            let end = seq.nodes[bracket as usize].last as usize;
            if seq.spans_lines(bracket) && seq.points[end].newlines == 0 {
                break_before(seq, end, "");
            }
            // Blank line before the next code after the bracket and the comma.
            let mut j = end + 1;
            while j < seq.blocks.len() - 1 && (seq.is_comment(j) || seq.blocks[j].kind == k::COMMA)
            {
                j += 1;
            }
            let gap = &seq.points[end + 1..=j];
            if j == seq.blocks.len() - 1 || gap.iter().any(|p| p.newlines >= 2) {
                continue;
            }
            if matches!(
                seq.blocks[j].text.to_ascii_uppercase().as_str(),
                "CYCLE" | "SEARCH"
            ) {
                continue;
            }
            if gap.iter().all(|p| p.newlines == 0) {
                // All on one line: the blank line goes after the comma, or before the code.
                let at = if seq.blocks[end + 1].kind == k::COMMA {
                    end + 2
                } else {
                    j
                };
                seq.points[at].newlines = 2;
                seq.points[at].ws = Cow::Borrowed("");
            } else {
                let mut at = j;
                while at > end + 1 && seq.is_comment(at - 1) && seq.points[at].newlines > 0 {
                    at -= 1;
                }
                if seq.points[at].newlines == 0 {
                    at = j;
                }
                seq.points[at].newlines = seq.points[at].newlines.max(1) + 1;
            }
        }
    }
}

/// The most line breaks (one more than blank lines) the gap before block `i` may have, by what
/// sqlfluff's LT15 calls its scope: inside a statement 1 blank line, between statements 2 and
/// between batches (after a T-SQL `GO`) 1. Leading and trailing comments of a statement lie
/// between statements.
fn max_breaks(seq: &Seq, i: usize) -> u32 {
    let common = seq.common_node(i - 1, i);
    let alone = seq.points[i - 1].newlines > 0 || i == 1;
    if alone && seq.blocks[i - 1].text.eq_ignore_ascii_case("go") {
        return 2;
    }
    let mut n = common;
    while n != NONE {
        let node = &seq.nodes[n as usize];
        if node.kind == k::STATEMENT {
            let inside = node.first != NONE && node.first < i as u32 && i as u32 <= node.last;
            let ctes = common != n && seq.nodes[common as usize].kind == k::WITH_COMPOUND_STATEMENT;
            return if inside && !ctes { 2 } else { 3 };
        }
        n = node.parent;
    }
    3
}

/// No blank lines at the start of the file; at most one inside a statement and two between
/// statements (LT13, LT15).
pub fn blank_lines(seq: &mut Seq) {
    seq.points[0].newlines = 0;
    seq.points[0].ws = Cow::Borrowed("");
    for i in 1..seq.blocks.len() {
        let max = max_breaks(seq, i);
        let p = &mut seq.points[i];
        p.newlines = p.newlines.min(max);
    }
}
