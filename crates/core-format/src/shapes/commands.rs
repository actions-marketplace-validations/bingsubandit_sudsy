//! A file that is a flat list of commands `name(arguments)`, one per line, each at the column its
//! block depth gives (the pairing of `core_syntax::block_pair` over the command names). A command
//! the language knows (an option's word list) has its arguments on the line when they fit, else
//! one per line with the closing parenthesis alone; any other command is kept as written: its
//! lines move with the command's indent (never left), the parenthesis stands alone when the
//! arguments span lines. A blank line stays up to the option's limit, but none stands right
//! after a block opener or arm or before an arm or closer; comments in front of an arm or
//! closer stay at the depth of the block body.
//!
//! concept: flat-command-list
//! what: A flat list of name(arguments) commands at their block depth: known commands fit on a line or break one argument per line, the others stay as written.
//! tools: gersemi indent, gersemi unknown commands
//! reads: rules:layout, path:block_pair::Block, option:known_commands

use super::{
    ListLayout, ListStyle, Rendered, TrailingComma, delimited_list_around, gap, leading_info,
};
use crate::FormatCtx;
use crate::case::Cases;
use crate::comments;
use crate::doc::{self, Doc, Outdent};
use crate::error::FormatError;
use crate::ignore::{self, IgnoreMark};
use core_syntax::block_pair::{Block, Pairing, Role, Step};
use core_syntax::{SyntaxElement, SyntaxNode, SyntaxToken};
use std::collections::HashMap;

type R = Result<Doc, FormatError>;

/// What the language says: the kind of a command node, the option with the names of the commands
/// it knows, its block words and its case rows.
pub struct Commands<'a> {
    pub blocks: &'static [Block],
    pub cases: &'a Cases,
    /// Blank lines kept between two commands.
    pub max_blank: usize,
}

/// `d` `levels` levels in.
fn indented(levels: u32, d: Doc) -> Doc {
    (0..levels).fold(d, |d, _| doc::indent(d))
}

fn significant(cx: &FormatCtx<'_>, node: &SyntaxNode) -> Vec<SyntaxToken> {
    (node.children_with_tokens())
        .filter_map(SyntaxElement::into_token)
        .filter(|t| !cx.grammar.is_trivia(t.kind()))
        .collect()
}

/// The known commands by lower-case name, with their spelling.
type Known<'a> = HashMap<String, &'a str>;

/// The name as written: a known command in its spelling, any other by the case rows.
fn written(spec: &Commands<'_>, known: &Known<'_>, name: &SyntaxToken) -> String {
    match known.get(&name.text().to_ascii_lowercase()) {
        Some(spelling) => (*spelling).to_string(),
        None => {
            (spec.cases.fixed(name.kind(), name.text())).unwrap_or_else(|| name.text().to_string())
        }
    }
}

/// The name of the command (its first token) and the ends of its arguments, `None` for a command
/// that lacks its `(` or `)`.
struct Parts {
    name: SyntaxToken,
    open: SyntaxToken,
    args: SyntaxNode,
    close: SyntaxToken,
}

fn parts(cx: &FormatCtx<'_>, node: &SyntaxNode) -> Option<Parts> {
    let toks = significant(cx, node);
    let args = node.children().next()?;
    match toks.as_slice() {
        [name, open, .., close] if close.text() == ")" && open.text() == "(" => Some(Parts {
            name: name.clone(),
            open: open.clone(),
            args,
            close: close.clone(),
        }),
        _ => None,
    }
}

fn known_style() -> ListStyle {
    ListStyle {
        separator: "",
        layout: ListLayout::Fit,
        trailing_comma: TrailingComma::Never,
        max_blank_lines: 0,
        ..ListStyle::default()
    }
}

/// A command whose arguments the language lays out.
fn laid_out(cx: &FormatCtx<'_>, name: String, node: &SyntaxNode, p: &Parts) -> R {
    let list = delimited_list_around(cx, node, &p.args, &p.open, &p.close, &known_style())?;
    Ok(doc::concat(vec![
        cx.leading_comments_doc(&p.name),
        doc::text(name),
        cx.trailing_comments_doc(&p.name),
        list,
    ]))
}

/// A command kept as written, moved to the column `to` (it stood at `from`): the lines after the
/// first move right by the difference, except those that follow a line with a comment or part
/// of a text that spans lines. The closing parenthesis is alone on its line when the arguments
/// span lines.
fn kept(cx: &FormatCtx<'_>, name: String, node: &SyntaxNode, p: &Parts, to: usize) -> Doc {
    let shift = to.saturating_sub(comments::column(&p.name));
    let g = cx.grammar;
    let mut args = String::new();
    // Does the line so far hold no comment and no end of a text that spans lines? Does the line
    // after the last break move? Is the next token the first of its line?
    let (mut clean, mut moves, mut at_start) = (true, false, false);
    let mut pending = String::new();
    let (mut code, mut comment, mut spans) = (false, false, false);
    let mut inside = false;
    for t in node
        .descendants_with_tokens()
        .filter_map(SyntaxElement::into_token)
    {
        if t == p.open {
            inside = true;
            continue;
        }
        if t == p.close {
            break;
        }
        if !inside {
            continue;
        }
        let text = t.text();
        if g.has_tag(t.kind(), "newline") {
            args.push_str(&std::mem::take(&mut pending));
            args.push('\n');
            spans = true;
            (moves, clean, at_start) = (clean, true, true);
        } else if g.has_tag(t.kind(), "whitespace") {
            if at_start {
                pending.push_str(text);
            } else {
                args.push_str(text);
            }
        } else {
            if at_start {
                if moves {
                    args.push_str(&" ".repeat(shift));
                }
                args.push_str(&std::mem::take(&mut pending));
                at_start = false;
            }
            args.push_str(text);
            let is_comment = g.has_tag(t.kind(), "comment");
            code |= !is_comment;
            comment |= is_comment;
            clean &= !is_comment && !text.contains('\n');
            spans |= text.contains('\n');
        }
    }
    let body = if !code && !comment {
        "()".to_string()
    } else if spans {
        format!("({}\n{})", args.trim_end(), " ".repeat(to))
    } else {
        format!("({args})")
    };
    doc::concat(vec![
        cx.leading_comments_doc(&p.name),
        crate::context::verbatim_text(&format!("{name}{body}")),
        cx.trailing_comments_doc(&p.close),
    ])
}

fn first_word(cx: &FormatCtx<'_>, node: &SyntaxNode) -> Option<SyntaxToken> {
    cx.first_token(node)
}

/// The file `node`: its commands one per line.
pub fn command_list(cx: &FormatCtx<'_>, node: &SyntaxNode, spec: &Commands<'_>) -> R {
    let g = cx.grammar;
    let size = cx.options.uint(core_config::shared::INDENT_SIZE, 4);
    let items: Vec<SyntaxNode> = node.children().collect();
    let mut pairing = Pairing::new(spec.blocks);
    let mut depth = 0;
    let steps: Vec<Step> = items
        .iter()
        .map(|n| {
            let step = match first_word(cx, n) {
                Some(t) => pairing.word(t.text(), usize::from(t.text_range().start()) as u32),
                None => Step {
                    role: Role::Plain,
                    depth,
                },
            };
            depth = step.depth;
            step
        })
        .collect();
    // The depth of the comments in front of each item: that of the body the previous one leaves.
    let after: Vec<u32> = std::iter::once(0)
        .chain(steps.iter().map(|s| s.after()))
        .collect();
    let known: Known<'_> = {
        cx.options
            .strings(core_config::opt::KNOWN_COMMANDS)
            .map(|s| (s.to_ascii_lowercase(), s))
            .collect()
    };
    let mut parts_out: Vec<Doc> = Vec::new();
    let mut prev: Option<(Rendered, Step)> = None;
    let mut i = 0;
    while i < items.len() {
        let step = steps[i];
        let (blank, mark, _) = leading_info(cx, &items[i])?;
        let comment_depth = after[i];
        let start = first_word(cx, &items[i]).ok_or_else(no_tokens)?;
        let (j, doc, verbatim, level) = if mark == Some(IgnoreMark::Start) {
            let j = ignore::region_end(items.len(), i, |n| {
                Ok::<_, FormatError>(leading_info(cx, &items[n])?.1)
            })? - 1;
            let last = cx.last_token(&items[j]).ok_or_else(no_tokens)?;
            (j, cx.verbatim_range(&start, &last), true, step.depth)
        } else if mark == Some(IgnoreMark::Ignore) {
            (i, cx.verbatim_node(&items[i]), true, step.depth)
        } else {
            let held =
                comment_depth > step.depth && !comments::leading_comments(g, &start).is_empty();
            if held {
                cx.hold_leading(&start);
            }
            let body = match parts(cx, &items[i]) {
                Some(p) => {
                    let name = written(spec, &known, &p.name);
                    if known.contains_key(&p.name.text().to_ascii_lowercase()) {
                        laid_out(cx, name, &items[i], &p)?
                    } else {
                        kept(cx, name, &items[i], &p, step.column(size))
                    }
                }
                None => cx.verbatim_node(&items[i]),
            };
            let doc = if held {
                let by = comment_depth - step.depth;
                doc::concat(vec![
                    cx.held_leading(&start),
                    Doc::Outdent(Outdent::Levels(by as usize), Box::new(body)),
                ])
            } else {
                body
            };
            (i, doc, false, if held { comment_depth } else { step.depth })
        };
        let edge = prev
            .as_ref()
            .is_some_and(|(_, p)| matches!(p.role, Role::Open | Role::Arm))
            || matches!(step.role, Role::Arm | Role::Close);
        let item = Rendered {
            first: items[i].clone(),
            last: items[j].clone(),
            blank_before: if edge { 0 } else { blank },
            doc,
            verbatim,
            sep: None,
            hole: false,
            tail: None,
        };
        let lead = match &prev {
            Some((p, _)) => gap(cx, p, &item, doc::hardline(), spec.max_blank, None, false),
            None => Doc::Nil,
        };
        parts_out.push(indented(level, doc::concat(vec![lead, item.doc.clone()])));
        let last_step = steps[j];
        prev = Some((item, last_step));
        i = j + 1;
    }
    let tail_depth = after[items.len()];
    let eof = cx
        .last_token(node)
        .or_else(|| node.last_token())
        .ok_or_else(|| FormatError::Shape("File without tokens".into()))?;
    let trailing = comments::leading_comments(g, &eof);
    if !trailing.is_empty() {
        let mut tail = Vec::new();
        if !items.is_empty() {
            tail.push(doc::hardline());
            if comments::blank_lines_before(g, &eof) > 0 {
                tail.push(doc::hardline());
            }
        }
        tail.push(crate::context::leading_comments_doc(&trailing));
        parts_out.push(indented(tail_depth, doc::concat(tail)));
    }
    Ok(doc::concat(parts_out))
}

fn no_tokens() -> FormatError {
    FormatError::Shape("item without tokens".into())
}
