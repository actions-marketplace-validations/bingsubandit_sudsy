//! Inline children and words laid out in a fill: each part is a word or a printed node (content)
//! or the gap between two of them (soft, breakable, hard, or a kept space), and the run is filled
//! to the line width. The gaps next to each other are merged (a hard line wins over a soft one),
//! the ends are trimmed, and a kept space prints as a space, or as its `marker` and a soft line
//! where the line breaks.
//!
//! concept: text-fill
//! what: Words and separators laid out in a fill: JSX children, inline markup text, prose.
//! tools: Prettier fill, markdown proseWrap

use crate::doc::{self, Doc};

/// One part of a run of inline children.
#[derive(Clone)]
pub enum Part {
    /// Placeholder that keeps content and gaps alternating; merged away by [`fill_children`].
    Empty,
    Hard,
    Soft,
    Line,
    /// Whitespace that is content (JSX `{" "}`): a space, or the marker and a line break when
    /// the line breaks.
    Space,
    Word(String),
    Doc(Doc),
}

impl Part {
    fn blank(&self) -> bool {
        matches!(self, Part::Empty | Part::Hard | Part::Soft | Part::Line)
    }
}

/// `text` split at runs of whitespace, the runs kept: words at even places, the first and the
/// last possibly empty.
pub fn runs(text: &str, space: impl Fn(char) -> bool + Copy) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut in_space = false;
    for c in text.chars() {
        if space(c) != in_space {
            in_space = !in_space;
            out.push(String::new());
        }
        out.last_mut().expect("never empty").push(c);
    }
    if in_space {
        out.push(String::new());
    }
    out
}

/// The words of `text` as a fill, a line between neighbours; `first` and `last` are attached to
/// the first and the last word.
pub fn words(text: &str, space: impl Fn(char) -> bool + Copy, first: &str, last: &str) -> Doc {
    let words: Vec<&str> = text.split(space).filter(|w| !w.is_empty()).collect();
    let end = words.len().saturating_sub(1);
    let mut parts = Vec::new();
    for (i, w) in words.iter().enumerate() {
        if i > 0 {
            parts.push(doc::line());
        }
        let head = if i == 0 { first } else { "" };
        let tail = if i == end { last } else { "" };
        if head.is_empty() && tail.is_empty() {
            parts.push(doc::snippet(w));
        } else {
            let mut word = String::with_capacity(head.len() + w.len() + tail.len());
            word.push_str(head);
            word.push_str(w);
            word.push_str(tail);
            parts.push(doc::text(word));
        }
    }
    doc::fill(parts)
}

/// Runs of `blank` characters become one space; a run at either end stays as one space.
pub fn collapse_blanks(text: &str, blank: impl Fn(char) -> bool + Copy) -> String {
    let mut out = String::with_capacity(text.len());
    for (i, w) in text.split(blank).filter(|w| !w.is_empty()).enumerate() {
        if i > 0 || text.starts_with(blank) {
            out.push(' ');
        }
        out.push_str(w);
    }
    if text.ends_with(blank) && !out.is_empty() {
        out.push(' ');
    }
    out
}

/// A cleaned run of parts, printed on one line or as a fill.
pub struct Children {
    parts: Vec<Part>,
    marker: &'static str,
}

/// Merges the gaps `parts` has left next to each other and trims both ends. `text`: the run
/// holds words (a run of nodes only keeps its hard lines between them).
pub fn fill_children(mut parts: Vec<Part>, marker: &'static str, text: bool) -> Children {
    merge(&mut parts, text);
    while parts.last().is_some_and(Part::blank) {
        parts.pop();
    }
    while parts.len() > 1 && parts[0].blank() && parts[1].blank() {
        parts.drain(..2);
    }
    Children { parts, marker }
}

impl Children {
    fn space(&self) -> Doc {
        doc::if_break(
            doc::concat(vec![doc::lit(self.marker), doc::softline()]),
            doc::lit(" "),
        )
    }

    fn one(&self, p: &Part) -> Doc {
        match p {
            Part::Empty => Doc::Nil,
            Part::Hard => doc::hardline(),
            Part::Soft => doc::softline(),
            Part::Line => doc::line(),
            Part::Space => self.space(),
            Part::Word(w) => doc::text(w.clone()),
            Part::Doc(d) => d.clone(),
        }
    }

    /// Every part in order, for the layout that fits on one line.
    pub fn flat(&self) -> Vec<Doc> {
        self.parts.iter().map(|p| self.one(p)).collect()
    }

    /// The fill, and whether a hard line or a part that breaks forces the parent to break.
    pub fn filled(&self) -> (Vec<Doc>, bool) {
        let marker = || doc::lit(self.marker);
        let (parts, mut forced) = (&self.parts, false);
        let mut out = Vec::new();
        for (i, p) in parts.iter().enumerate() {
            if matches!(p, Part::Space) {
                let before_empty = i == 1 && matches!(parts[0], Part::Empty);
                if before_empty && parts.len() == 2 {
                    out.push(marker());
                    continue;
                }
                if before_empty {
                    out.push(doc::concat(vec![marker(), doc::hardline()]));
                    continue;
                }
                if i + 1 == parts.len() {
                    // Trailing: measured with the content before it.
                    let last = out.pop().unwrap_or(Doc::Nil);
                    out.push(doc::concat(vec![last, marker()]));
                    continue;
                }
                if i + 2 == parts.len()
                    && matches!(parts[i + 1], Part::Empty)
                    && i > 0
                    && matches!(parts[i - 1], Part::Hard)
                {
                    out.push(marker());
                    continue;
                }
            }
            out.push(self.one(p));
            forced |= match p {
                Part::Hard => true,
                Part::Doc(d) => d.will_break(),
                _ => false,
            };
        }
        (out, forced)
    }
}

/// Removes the doubled and misplaced gaps left between text and nodes.
fn merge(parts: &mut Vec<Part>, text: bool) {
    use Part::{Empty as E, Hard as H, Soft as S, Space as W};
    let mut i = parts.len() as isize - 2;
    while i >= 0 {
        let u = i as usize;
        let at = |j: usize| parts.get(j);
        let (a, b, c) = (at(u), at(u + 1), at(u + 2));
        let b_empty = matches!(b, Some(E));
        let pair_hard = matches!((a, c), (Some(H), Some(H))) && b_empty;
        let pair_empty = matches!(a, Some(E)) && b_empty;
        let line_ws = matches!(a, Some(H | S)) && b_empty && matches!(c, Some(W));
        let ws_line = matches!(a, Some(W)) && b_empty && matches!(c, Some(H | S));
        let double_ws = matches!(a, Some(W)) && b_empty && matches!(c, Some(W));
        // The hard line wins over a soft one.
        let soft_hard = b_empty && matches!((a, c), (Some(S), Some(H)));
        let hard_soft = b_empty && matches!((a, c), (Some(H), Some(S)));
        if (pair_hard && text) || pair_empty || line_ws || double_ws || soft_hard {
            parts.drain(u..u + 2);
        } else if ws_line || hard_soft {
            parts.drain(u + 1..u + 3);
        }
        i -= 1;
    }
}
