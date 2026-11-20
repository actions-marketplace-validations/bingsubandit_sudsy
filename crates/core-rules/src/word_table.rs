//! A table of the words of a language's dialects: one `WORD flags` line per word, one flag letter
//! per dialect, lower case when the dialect lets the word name something and upper case when it
//! reserves it. A language's `RuleHooks` answer `is_keyword` and `is_reserved` from it.
//!
//! concept: word-table
//! what: The keyword and reserved words of each dialect of a language, from a text table with a flag letter per dialect.
//! tools: sqlfluff RF04, RF06
//! reads: rule:keyword-identifier, rule:needless-quotes

use std::collections::HashMap;
use std::sync::OnceLock;

pub struct WordTable {
    text: &'static str,
    /// The dialect names (the file's version) and their flag letters; the first is the default.
    dialects: &'static [(&'static str, char)],
    words: OnceLock<HashMap<&'static str, &'static str>>,
}

impl WordTable {
    pub const fn new(text: &'static str, dialects: &'static [(&'static str, char)]) -> WordTable {
        WordTable {
            text,
            dialects,
            words: OnceLock::new(),
        }
    }

    fn flags(&self, upper: &str) -> Option<&'static str> {
        let words = self.words.get_or_init(|| {
            (self.text.lines())
                .filter_map(|l| l.split_once(' '))
                .collect()
        });
        words.get(upper).copied()
    }

    fn letter(&self, version: Option<&str>) -> char {
        (self.dialects.iter())
            .find(|(name, _)| Some(*name) == version)
            .unwrap_or(&self.dialects[0])
            .1
    }

    /// Does the dialect `version` (`None`: the default) reserve `upper` (the word in upper case)?
    pub fn reserved(&self, version: Option<&str>, upper: &str) -> bool {
        let l = self.letter(version).to_ascii_uppercase();
        self.flags(upper).is_some_and(|f| f.contains(l))
    }

    /// Is `upper` a keyword of the dialect that may still name something?
    pub fn keyword(&self, version: Option<&str>, upper: &str) -> bool {
        let l = self.letter(version).to_ascii_lowercase();
        self.flags(upper).is_some_and(|f| f.contains(l))
    }
}
