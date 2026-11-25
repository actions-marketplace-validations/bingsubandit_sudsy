//! PowerShell layout after PSScriptAnalyzer's `Invoke-Formatter` (its CodeFormatting settings):
//! every line is re-indented, and blanks and line breaks are added where it adds them (braces,
//! operators, separators, pipes, `=` of a hashtable). Blanks the source has are kept, and so are
//! the words, strings and line breaks the formatter has no rule for. The tree is flat, so the
//! formatter walks the tokens.
//!
//! concept: block-and-statements

use crate::generated::kind as k;
use core_config::shared;
use core_format::comments::CommentStyle;
use core_format::rules::LazyRules;
use core_format::stream::{self, Tk, multiline};
use core_format::{Doc, FormatCtx, FormatError, NodeFormatter, columns};
use core_syntax::{SyntaxKind, SyntaxNode, grammars};
use std::collections::HashMap;

include!("schema.generated.rs");

static ROWS: LazyRules = LazyRules::new(
    || grammars::POWERSHELL.get(),
    include_str!("../format.rules"),
);

pub struct PowerShellFormatter;

impl NodeFormatter for PowerShellFormatter {
    fn comment_style(&self, _: &core_config::ResolvedOptions) -> CommentStyle {
        // The blanks before a comment are the source's, none added.
        CommentStyle {
            gap: 0,
            keep_gap: true,
            trim_lines: true,
            ..Default::default()
        }
    }

    fn format_node(
        &self,
        node: &SyntaxNode,
        cx: &FormatCtx<'_>,
    ) -> Result<Option<Doc>, FormatError> {
        Ok((node.kind() == k::PROGRAM).then(|| program(cx, node)))
    }
}

/// Is it one of these keywords (PowerShell ignores their case)?
trait Keyword {
    fn keyword(&self, words: &[&str]) -> bool;
}

impl Keyword for Tk {
    fn keyword(&self, words: &[&str]) -> bool {
        self.kind == k::KEYWORD && words.iter().any(|w| self.text().eq_ignore_ascii_case(w))
    }
}

fn is_open(kind: SyntaxKind) -> bool {
    ROWS.get().brackets().is_open(kind)
}

/// Is token `i` directly inside the body of a `class` or `switch` (`which`)?
fn in_block(tks: &[Tk], i: usize, which: &str) -> bool {
    let mut j = i;
    let open = loop {
        if j == 0 {
            return false;
        }
        j -= 1;
        match tks[j].kind {
            k::CLOSE_BRACE | k::CLOSE_PAREN => match tks[j].pair {
                Some(o) => j = o,
                None => return false,
            },
            k::OPEN_BRACE => break j,
            _ if is_open(tks[j].kind) => return false,
            _ => {}
        }
    };
    // `class A : B {` and `switch -regex ($x) {`
    tks[open.saturating_sub(6)..open]
        .iter()
        .any(|t| t.keyword(&[which]))
}

/// Is the `{` at `o` the block of a statement (`if (...) {`, `else {`, `function f {`,
/// `catch [T] {`) and not a script block that is an expression or an argument
/// (`ForEach-Object {`, `$x = {`)?
fn statement_block(tks: &[Tk], o: usize) -> bool {
    if tks[o].kind != k::OPEN_BRACE || o == 0 {
        return false;
    }
    let prev = &tks[o - 1];
    match prev.kind {
        k::KEYWORD => prev.keyword(&[
            "else",
            "catch",
            "try",
            "finally",
            "do",
            "begin",
            "process",
            "end",
            "dynamicparam",
            "default",
        ]),
        // The condition of a control statement, the parameters of a function or the method of a
        // class (not `param(...)`).
        k::CLOSE_PAREN => prev.pair.is_some_and(|p| {
            p > 0
                && (tks[p - 1].keyword(&[
                    "if", "elseif", "while", "switch", "foreach", "for", "until", "catch",
                ]) || (p > 1
                    && tks[p - 1].kind == k::WORD
                    && (tks[p - 2].keyword(&["function", "filter"]) || in_block(tks, p, "class"))))
        }),
        // `catch [System.Exception], [System.IO.IOException] {`
        k::CLOSE_BRACKET => (tks[..o].iter().rev())
            .find(|t| !t.is(&[k::WORD, k::OPEN_BRACKET, k::CLOSE_BRACKET, k::COMMA]))
            .is_some_and(|t| t.keyword(&["catch"])),
        k::WORD => {
            (o > 1 && tks[o - 2].keyword(&["function", "filter", "class", "enum"]))
                || in_block(tks, o, "switch")
        }
        // The clause of a `switch`: `5 {`.
        k::STRING => in_block(tks, o, "switch"),
        _ => false,
    }
}

/// Is token `i` a bracket of a block that spans lines and whose braces the formatter places: a
/// statement block or a hashtable?
fn placed_block(tks: &[Tk], i: usize) -> bool {
    let Some(p) = tks[i].pair else {
        return false;
    };
    let o = i.min(p);
    multiline(tks, o) && (tks[o].kind == k::HASH_OPEN || statement_block(tks, o))
}

/// Where the output has line breaks: the source's, minus a brace that moves up to its head, plus
/// breaks after `{`, before `}` and before `else`/`catch`/`finally` of a block that spans lines.
fn place_breaks(tks: &mut [Tk]) {
    let mut joined = Vec::new();
    for i in 0..tks.len() {
        let mut nl = i > 0 && tks[i].breaks > 0;
        if i > 0 && !tks[i].cont {
            let prev = &tks[i - 1];
            match tks[i].kind {
                // `)` + line break + `{`: the brace joins the line of its head.
                k::OPEN_BRACE
                    if nl
                        && statement_block(tks, i)
                        && tks[i].leading.is_empty()
                        && !prev.trailing
                        && multiline(tks, i) =>
                {
                    nl = false;
                    joined.push(i);
                }
                // `{ a` + line break + `}`: the statements go to a line of their own.
                k::CLOSE_BRACE if !nl && placed_block(tks, i) && !prev.is(&[k::OPEN_BRACE]) => {
                    nl = true;
                }
                k::KEYWORD
                    if !nl
                        && tks[i].keyword(&["else", "elseif", "catch", "finally"])
                        && prev.kind == k::CLOSE_BRACE
                        && placed_block(tks, i - 1) =>
                {
                    nl = true;
                }
                _ => {}
            }
            if !nl
                && prev.kind == k::OPEN_BRACE
                && multiline(tks, i - 1)
                && statement_block(tks, i - 1)
            {
                nl = !tks[i].is(&[k::CLOSE_BRACE]);
            }
        }
        tks[i].newline = nl || tks[i].cont;
    }
    for i in joined {
        tks[i].gap.clear();
    }
}

/// For each `=` of a multi-line hashtable entry that starts its line: the blanks that line up
/// the `=` of all such entries.
fn align_hashtables(cx: &FormatCtx<'_>, root: &SyntaxNode, tks: &[Tk]) -> HashMap<usize, usize> {
    let by_start: HashMap<usize, usize> = (tks.iter().enumerate())
        .map(|(i, t)| (usize::from(t.tok.text_range().start()), i))
        .collect();
    let mut pads = HashMap::new();
    for hash in root.descendants().filter(|n| n.kind() == k::HASH) {
        let Some(open) = cx.first_token(&hash) else {
            continue;
        };
        let Some(&oi) = by_start.get(&usize::from(open.text_range().start())) else {
            continue;
        };
        if !multiline(tks, oi) {
            continue;
        }
        // (key width, index of `=`) of the entries that start a line and have `=` on it.
        let mut entries = Vec::new();
        for entry in hash.children().filter(|n| n.kind() == k::HASH_ENTRY) {
            let (Some(first), Some(eq)) = (
                cx.first_token(&entry),
                entry
                    .children_with_tokens()
                    .filter_map(|e| e.into_token())
                    .find(|t| t.kind() == k::ASSIGN),
            ) else {
                continue;
            };
            let (Some(&fi), Some(&ei)) = (
                by_start.get(&usize::from(first.text_range().start())),
                by_start.get(&usize::from(eq.text_range().start())),
            ) else {
                continue;
            };
            if !tks[fi].newline || tks[fi].first_line != tks[ei].first_line {
                continue;
            }
            let end = usize::from(tks[ei - 1].tok.text_range().end());
            let width = end - usize::from(first.text_range().start());
            entries.push((width, ei));
        }
        columns::pad(
            &mut entries,
            |e| e.0,
            |e, pad| {
                pads.insert(e.1, pad);
            },
        );
    }
    pads
}

/// How the blanks between `prev` and `cur` (on one line) are set: `None` keeps the source's,
/// `Some(false)` adds one where there is none, `Some(true)` makes it exactly one.
fn blank(tks: &[Tk], i: usize, pads: &HashMap<usize, usize>) -> Option<bool> {
    let (prev, cur) = (&tks[i - 1], &tks[i]);
    let rows = ROWS.get().space();
    if rows.at_least_one(&prev.tok, &cur.tok) {
        return Some(false);
    }
    // Invoke-Formatter does not add the blank after the aligned `=` of `a=1`.
    if prev.kind == k::ASSIGN && pads.contains_key(&(i - 1)) && cur.gap.is_empty() {
        return None;
    }
    // `$x.ForEach{ }`: the block is the argument of the method.
    let method = prev.kind == k::WORD && (prev.text().contains('.') || prev.text().contains("::"));
    if cur.kind == k::OPEN_BRACE && method {
        return None;
    }
    rows.spaced(&prev.tok, &cur.tok).then_some(true)
}

fn program(cx: &FormatCtx<'_>, root: &SyntaxNode) -> Doc {
    let rules = ROWS.get().stream();
    let mut tks = stream::collect(cx.grammar, root, rules);
    stream::pair_brackets(&mut tks, &rules.brackets);
    place_breaks(&mut tks);
    let pads = align_hashtables(cx, root, &tks);
    stream::emit(cx, rules, tks, &[k::PIPE], |tks, i| spacing(tks, i, &pads))
}

/// The blanks between token `i - 1` and `i` on one line.
fn spacing(tks: &[Tk], i: usize, pads: &HashMap<usize, usize>) -> String {
    if let Some(pad) = pads.get(&i) {
        return " ".repeat(pad + 1);
    }
    match blank(tks, i, pads) {
        Some(true) => " ".into(),
        Some(false) if tks[i].gap.is_empty() => " ".into(),
        _ => tks[i].gap.clone(),
    }
}
