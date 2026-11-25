//! Template actions in markup (`{{ if .x }}` of Go templates and the like): text a parser keeps
//! as written, with no markup of its own. A block of actions (an opener word up to a closer
//! word) must open and close inside one element, so the tree of the page without the actions is
//! the tree of every page they produce. The language gives its delimiters and words as an
//! [`Actions`] value; the shared code finds where an action ends, skips actions inside a quoted
//! value, classifies an action by its first word and counts the open blocks of each owner.
//!
//! concept: template-actions
//! what: Delimited template actions in markup, their blocks counted per owning element.
//! tools: html-eslint indent
//! reads: option:dialect

use std::collections::HashMap;

/// What an action does to the blocks of its owner.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Step {
    Open,
    Continue,
    Close,
}

/// The delimiters and block words of a template language.
pub struct Actions {
    pub open: &'static str,
    pub close: &'static str,
    /// Written right after the opening delimiter to trim whitespace (`-` of `{{- x }}`).
    pub trim: char,
    pub openers: &'static [&'static str],
    pub continuers: &'static [&'static str],
    pub closers: &'static [&'static str],
}

impl Actions {
    /// Does `src` have an action (its opening delimiter)?
    pub fn occurs_in(&self, src: &str) -> bool {
        src.contains(self.open)
    }

    /// The end of the action that starts at `at` (after its closing delimiter), when it closes.
    pub fn end(&self, src: &str, at: usize) -> Option<usize> {
        let rest = src.get(at..)?.strip_prefix(self.open)?;
        rest.find(self.close)
            .map(|i| at + self.open.len() + i + self.close.len())
    }

    /// What the action `inner` (the text between the delimiters) does: its first word, after the
    /// trim marker, names it.
    pub fn step(&self, inner: &str) -> Option<Step> {
        let inner = inner.trim_start_matches(self.trim).trim_start();
        let word = inner.split(|c: char| !c.is_ascii_alphabetic()).next()?;
        let has = |words: &[&str]| words.contains(&word);
        if has(self.openers) {
            Some(Step::Open)
        } else if has(self.continuers) {
            Some(Step::Continue)
        } else if has(self.closers) {
            Some(Step::Close)
        } else {
            None
        }
    }

    /// The end of a quoted value after the closing `quote` (`from` is after the opening one);
    /// an action inside is skipped, so a quote it holds does not end the value.
    pub fn value_end(&self, src: &str, from: usize, quote: u8) -> Option<usize> {
        let b = src.as_bytes();
        let mut i = from;
        while i < b.len() {
            if let Some(end) = self.end(src, i) {
                i = end;
            } else if b[i] == quote {
                return Some(i + 1);
            } else {
                i += 1;
            }
        }
        None
    }

    /// The pieces of `text`, `(is an action, text)`, in order.
    pub fn pieces<'a>(&self, text: &'a str) -> Vec<(bool, &'a str)> {
        let mut out = Vec::new();
        let (mut from, mut at) = (0, 0);
        while at < text.len() {
            match (text.is_char_boundary(at), self.end(text, at)) {
                (true, Some(end)) => {
                    if from < at {
                        out.push((false, &text[from..at]));
                    }
                    out.push((true, &text[at..end]));
                    from = end;
                    at = end;
                }
                _ => at += 1,
            }
        }
        if from < text.len() {
            out.push((false, &text[from..]));
        }
        out
    }
}

/// The open blocks of each owner (an element, by an id of the parser's; the root has its own)
/// and where the last one opened.
#[derive(Default)]
pub struct Blocks(HashMap<usize, (i32, usize)>);

impl Blocks {
    /// Counts an action at `at` in `owner`; `false` when it continues or closes a block the
    /// owner has not opened.
    pub fn step(&mut self, owner: usize, at: usize, step: Step) -> bool {
        let entry = self.0.entry(owner).or_insert((0, at));
        match step {
            Step::Open => *entry = (entry.0 + 1, at),
            Step::Continue | Step::Close if entry.0 == 0 => return false,
            Step::Continue => {}
            Step::Close => entry.0 -= 1,
        }
        true
    }

    /// Where the blocks still open began.
    pub fn unclosed(&self) -> Vec<usize> {
        self.0
            .values()
            .filter(|&&(n, _)| n != 0)
            .map(|&(_, at)| at)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GO: Actions = Actions {
        open: "{{",
        close: "}}",
        trim: '-',
        openers: &["if", "range"],
        continuers: &["else"],
        closers: &["end"],
    };

    #[test]
    fn ends_and_steps() {
        assert_eq!(GO.end("a {{ x }} b", 2), Some(9));
        assert_eq!(GO.end("a {{ x", 2), None);
        assert_eq!(GO.step("- if .x"), Some(Step::Open));
        assert_eq!(GO.step(" end "), Some(Step::Close));
        assert_eq!(GO.step(" .x "), None);
    }

    #[test]
    fn value_skips_actions() {
        let s = r#"{{ "a" }}b" c"#;
        assert_eq!(GO.value_end(s, 0, b'"'), Some(s.len() - 2));
    }

    #[test]
    fn pieces_split() {
        assert_eq!(
            GO.pieces("a{{ b }}c"),
            vec![(false, "a"), (true, "{{ b }}"), (false, "c")]
        );
    }

    #[test]
    fn blocks_count_per_owner() {
        let mut b = Blocks::default();
        assert!(b.step(1, 0, Step::Open));
        assert!(!b.step(2, 5, Step::Close));
        assert_eq!(b.unclosed(), vec![0]);
        assert!(b.step(1, 9, Step::Close));
        assert!(b.unclosed().is_empty());
    }
}
