//! Code of another language inside a file (a Dockerfile's `RUN`, a Markdown fence), as the host's
//! provider reports it. The engine parses the region with the other language's provider.

use crate::{TextRange, TextSize};

/// The `TextSize` of a byte offset or length.
pub fn size(n: usize) -> TextSize {
    TextSize::try_from(n).expect("a file is under 4 GiB")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Embedded {
    /// Names the language: its name or a file extension (a fence's info word).
    pub language: String,
    /// Where the code is, in host offsets.
    pub range: TextRange,
    /// The parts of `range` that belong to the host, in order: the container prefix of each fence
    /// line, the comment lines inside a continued `RUN`.
    pub carved: Vec<TextRange>,
    /// Whether the host lets the language's formatter rewrite the code.
    pub format: bool,
    /// Whether the code is parsed without the carved parts (a language that reads indentation
    /// needs the fence's container prefix gone) instead of with them blanked; offsets of the
    /// parse then go through `to_host`.
    pub strip: bool,
    /// Whether the language's own rules run on the code. A host that reads the parse itself (a Vue
    /// `<script>`, whose names the template uses) leaves it off.
    pub lint: bool,
}

impl Embedded {
    /// A region the host does not let the formatter rewrite. `carved` may be in any order and
    /// overlap; it is clipped to `range`, sorted and merged.
    pub fn new(language: &str, range: TextRange, mut carved: Vec<TextRange>) -> Self {
        carved.retain_mut(|c| match c.intersect(range) {
            Some(r) => {
                *c = r;
                true
            }
            None => false,
        });
        carved.sort_by_key(|c| c.start());
        let mut merged: Vec<TextRange> = Vec::new();
        for c in carved {
            match merged.last_mut() {
                Some(last) if c.start() <= last.end() => *last = last.cover(c),
                _ => merged.push(c),
            }
        }
        Embedded {
            language: language.into(),
            range,
            carved: merged,
            format: false,
            strip: false,
            lint: true,
        }
    }

    /// The region's text with the carved parts blanked: as long as the region, so an offset in
    /// the result is an offset from `range.start()`.
    pub fn blanked(&self, text: &str) -> String {
        let mut out = text[self.range].to_string();
        for c in &self.carved {
            let at = usize::from(c.start() - self.range.start())
                ..usize::from(c.end() - self.range.start());
            out.replace_range(at.clone(), &" ".repeat(at.len()));
        }
        out
    }

    /// The text to parse: stripped or blanked, as the region says.
    pub fn code(&self, text: &str) -> String {
        if self.strip {
            self.stripped(text)
        } else {
            self.blanked(text)
        }
    }

    /// The host offset of `at`, an offset in `code()`; `end` for the end of a range, which stays
    /// before a carved part that starts there.
    pub fn to_host(&self, at: TextSize, end: bool) -> TextSize {
        let mut host = self.range.start() + at;
        if self.strip {
            let mut removed = TextSize::from(0);
            for c in &self.carved {
                let here = c.start() - self.range.start() - removed;
                if at > here || (at == here && !end) {
                    removed += c.len();
                } else {
                    break;
                }
            }
            host += removed;
        }
        host
    }

    /// The region's text without the carved parts: the code as it reads outside the host.
    pub fn stripped(&self, text: &str) -> String {
        let (mut out, mut from) = (String::new(), self.range.start());
        for c in &self.carved {
            out.push_str(&text[TextRange::new(from, c.start())]);
            from = c.end();
        }
        out + &text[TextRange::new(from, self.range.end())]
    }
}
