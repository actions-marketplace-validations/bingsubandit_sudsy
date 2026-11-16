//! What goes between the tokens: line breaks, blank lines, comments, and whether a blank
//! separates two tokens of a line.

use super::src::{Piece, Source};
use super::{blank, space};
use core_format::FormatCtx;

pub(super) enum Item {
    /// Line breaks: 1 ends the line, 2 also leaves a blank line.
    Break(u32),
    /// A comment and, if it starts a line, the blanks before it as written.
    Comment(String, Option<String>),
    /// The trivia of a gap inside a string, as written.
    Raw(String),
}

pub(super) struct Plan {
    pub items: Vec<Vec<Item>>,
    pub space_before: Vec<bool>,
}

impl Plan {
    #[allow(clippy::needless_range_loop)]
    pub fn new(src: &Source, _cx: &FormatCtx<'_>) -> Self {
        let n = src.toks.len();
        let mut items = Vec::with_capacity(n + 1);
        let mut space_before = vec![false; n + 1];
        for i in 0..=n {
            let gap = &src.gaps[i];
            let mut gap_items = Vec::new();
            if src.gap_in_string(i) {
                gap_items.push(Item::Raw(gap.raw.clone()));
            } else {
                for (j, piece) in gap.pieces.iter().enumerate() {
                    match piece {
                        Piece::Nl(c) => gap_items.push(Item::Break((*c).min(2))),
                        Piece::Comment(t) => {
                            let own_line =
                                |q: usize| matches!(gap.pieces.get(q), Some(Piece::Nl(_)));
                            let indent = match j.checked_sub(1).map(|q| (q, &gap.pieces[q])) {
                                Some((q, Piece::Ws(w)))
                                    if (q == 0 && i == 0) || (q > 0 && own_line(q - 1)) =>
                                {
                                    Some(w.clone())
                                }
                                Some((_, Piece::Nl(_))) => Some(String::new()),
                                None if i == 0 => Some(String::new()),
                                _ => None,
                            };
                            gap_items.push(Item::Comment(t.clone(), indent));
                        }
                        Piece::Ws(_) => {}
                    }
                }
                if let Some(Item::Comment(t, _)) = gap_items.last()
                    && (t.starts_with("//") || t.starts_with("#!"))
                {
                    gap_items.push(Item::Break(1));
                }
            }
            if i > 0 && i < n {
                space_before[i] = match gap_items.last() {
                    None => space::want(&src.toks[i - 1], &src.toks[i])
                        .unwrap_or(!gap.pieces.is_empty()),
                    Some(Item::Comment(..)) => matches!(gap.pieces.last(), Some(Piece::Ws(_))),
                    _ => false,
                };
            }
            items.push(gap_items);
        }
        blank::apply(src, &mut items);
        Plan {
            items,
            space_before,
        }
    }

    /// The output line of each token.
    #[allow(clippy::needless_range_loop)]
    pub fn lines(&self, src: &Source) -> Vec<u32> {
        let n = src.toks.len();
        let mut lines = vec![0; n];
        let mut line = 0;
        for i in 0..=n {
            for item in &self.items[i] {
                line += match item {
                    Item::Break(c) => *c,
                    Item::Comment(t, _) | Item::Raw(t) => t.matches('\n').count() as u32,
                };
            }
            if i < n {
                lines[i] = line;
                line += src.toks[i].text().matches('\n').count() as u32;
            }
        }
        lines
    }
}
