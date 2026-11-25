//! `heading-style`: headings of a document are all written one way, `#` in front of the text (ATX)
//! or text over a line of `=` or `-` (setext). Style `consistent` takes the first heading's, the
//! others name it. The fix writes a heading of one line, level 1 or 2, in the style wanted. The
//! nodes are those of the grammar kind the option `kind` names; a heading is ATX when a run of one
//! to six `#` stands in front of its text on its line, else setext, whose level comes from the
//! underline line (`=` is 1).
//!
//! concept: heading-style
//! what: Headings written in one style, ATX or setext, with a fix both ways for one-line headings.
//! tools: markdownlint MD003

use crate::diagnostic::Edit;
use crate::options::{Check, Opt, OptionDecl};
use crate::rule;
use crate::rule::{FileContext, RuleContext};
use core_syntax::{SyntaxNode, SyntaxToken, TextRange, TextSize};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Style {
    Atx,
    Setext,
}

impl Style {
    fn name(self) -> &'static str {
        match self {
            Style::Atx => "atx",
            Style::Setext => "setext",
        }
    }
}

/// A heading as written: its style and level, and the range from its first character to the end
/// of its last line when it can be rewritten (one line of text, at most three spaces of
/// indentation, nothing else on its lines).
struct Heading {
    style: Style,
    level: usize,
    text: String,
    /// The whole heading, markers included.
    span: Option<TextRange>,
}

rule! {
    pub HeadingStyle {
        id: "heading-style",
        category: Style,
        default: Off,
        fix: Safe,
        description: "Reports a heading written in another style than the document's: `#` before the \
              text (ATX) or the text over a line of `=` or `-` (setext). The option `style` is \
              `atx`, `setext` or `consistent` (the style of the first heading, the default). The \
              fix rewrites a one-line heading of level 1 or 2.",
        why: "Two heading styles in one document make it look edited by different hands, and \
              tools that search for headings (outlines, scripts) must know both. Only level 1 \
              and 2 have a setext form, so ATX is the one style that works for every level.\n\
              To fix, use the style of the document; the fix does it for level 1 and 2.",
        example: "Title\n=====\n\n## Section   // reported at `Section` with style consistent",
        options: [
            OptionDecl {
                name: "style",
                default: Opt::Str("consistent"),
                check: Check::OneOf(&["consistent", "atx", "setext"]),
                description: "`atx`, `setext`, or `consistent` (the first heading's style).",
            },
            OptionDecl {
                name: "kind",
                default: Opt::Str("Heading"),
                check: Check::Any,
                description: "The name of the grammar kind of a heading node.",
            },
        ],
    }

    fn check_file(&self, file: &FileContext<'_>, ctx: &mut RuleContext<'_>) {
        let g = file.grammar;
        let kind = ctx.options.string("kind").to_string();
        let wanted = match ctx.options.string("style") {
            "atx" => Some(Style::Atx),
            "setext" => Some(Style::Setext),
            _ => None,
        };
        let mut first = wanted;
        let nodes = file
            .root
            .descendants()
            .filter(|n| g.def(n.kind()).is_some_and(|d| d.name == kind));
        for n in nodes {
            let Some((head, last)) = text_ends(file, &n) else {
                continue;
            };
            let h = read(file.text, &head, &last);
            let expected = *first.get_or_insert(h.style);
            if h.style == expected {
                continue;
            }
            let fix = rewrite(&h, expected);
            ctx.report_fix(
                head.text_range(),
                format!("heading style is {}; expected {}", h.style.name(), expected.name()),
                fix.map(|e| ("Rewrite the heading", e)),
            );
        }
    }
}

/// The first and last token of a heading's text.
fn text_ends(file: &FileContext<'_>, n: &SyntaxNode) -> Option<(SyntaxToken, SyntaxToken)> {
    let mut tokens = n
        .descendants_with_tokens()
        .filter_map(|e| e.into_token())
        .filter(|t| !file.grammar.is_trivia(t.kind()));
    let first = tokens.next()?;
    Some((first.clone(), tokens.last().unwrap_or(first)))
}

fn range(from: usize, to: usize) -> TextRange {
    TextRange::new(TextSize::from(from as u32), TextSize::from(to as u32))
}

/// The style and level of the heading whose text runs from `head` to `last`.
fn read(text: &str, head: &SyntaxToken, last: &SyntaxToken) -> Heading {
    let (from, end) = (
        usize::from(head.text_range().start()),
        usize::from(last.text_range().end()),
    );
    let line = text[..from].rfind('\n').map_or(0, |i| i + 1);
    let before = text[line..from].trim_end();
    let hashes = before.chars().rev().take_while(|c| *c == '#').count();
    let rest = &before[..before.len() - hashes];
    let one_line = !text[from..end].contains('\n');
    if (1..=6).contains(&hashes) && (rest.is_empty() || rest.ends_with([' ', '\t', '>'])) {
        let line_end = end + text[end..].find('\n').unwrap_or(text.len() - end);
        let plain = rest.chars().all(|c| c == ' ') && rest.len() <= 3;
        let body = closing_removed(text[from..end].trim_end());
        return Heading {
            style: Style::Atx,
            level: hashes,
            text: body.to_string(),
            span: (one_line && plain && text[end..line_end].trim_matches([' ', '\t']).is_empty())
                .then(|| range(line + (before.len() - hashes), line_end)),
        };
    }
    let rest = text[end..].strip_prefix('\n');
    let underline = rest.and_then(|r| r.split('\n').next()).unwrap_or("");
    let bare = underline.trim_matches([' ', '\t']);
    let mark = bare.chars().next().unwrap_or(' ');
    let level = if mark == '=' { 1 } else { 2 };
    let valid = !bare.is_empty()
        && bare.chars().all(|c| c == mark)
        && underline.len() - underline.trim_start_matches(' ').len() <= 3;
    let plain = text[line..from].chars().all(|c| c == ' ') && from - line <= 3;
    Heading {
        style: Style::Setext,
        level,
        text: text[from..end].to_string(),
        span: (one_line && valid && plain).then(|| range(from, end + 1 + underline.len())),
    }
}

/// The text of an ATX heading without its closing sequence of `#`.
fn closing_removed(text: &str) -> &str {
    let body = text.trim_end_matches('#');
    if body.len() < text.len() && (body.is_empty() || body.ends_with(' ')) {
        body.trim_end()
    } else {
        text
    }
}

/// The edit that writes a heading in `style`, for level 1 and 2 only.
fn rewrite(h: &Heading, style: Style) -> Option<Vec<Edit>> {
    let span = h
        .span
        .filter(|_| h.level <= 2 && !h.text.is_empty() && !h.text.ends_with('#'))?;
    let new = match style {
        Style::Atx => format!("{} {}", "#".repeat(h.level), h.text),
        Style::Setext => {
            let mark = if h.level == 1 { '=' } else { '-' };
            let width = h.text.chars().count().max(3);
            format!("{}\n{}", h.text, mark.to_string().repeat(width))
        }
    };
    Some(vec![Edit::replace(span, new)])
}
