//! Word tables of a language's data files: one row per line, `key|word word ..`, empty lines and
//! `#` comment lines skipped. The key is everything before the last `|`, so a key may hold a `|`
//! itself (`postgres|COPY`). A table of word lists read by name is a [`WordLists`].
//!
//! concept: word-table-rows
//! what: Rows of `key|words` in a data file, read once into lists by key.
//! tools: none (parser data)

use std::collections::HashSet;

/// The rows of `text`: key and the words after the last `|`.
pub fn rows(text: &'static str) -> impl Iterator<Item = (&'static str, &'static str)> {
    text.lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.rsplit_once('|'))
}

/// The words of `text`, one per line (empty lines and `#` comments skipped).
pub fn lines(text: &'static str) -> HashSet<&'static str> {
    text.lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect()
}

/// The rows of a table as word sets by key.
pub fn sets(text: &'static str) -> impl Iterator<Item = (&'static str, HashSet<&'static str>)> {
    rows(text).map(|(key, words)| (key, words.split(' ').collect()))
}

/// The word lists of a table, looked up by key.
pub struct WordLists(Vec<(&'static str, Vec<&'static str>)>);

impl WordLists {
    pub fn parse(text: &'static str) -> Self {
        Self(
            rows(text)
                .map(|(key, words)| (key, words.split(' ').collect()))
                .collect(),
        )
    }

    /// The words of the row `key` (none when the table has no such row).
    pub fn get(&self, key: &str) -> &[&'static str] {
        let found = self.0.iter().find(|(k, _)| *k == key);
        debug_assert!(found.is_some(), "no word list {key}");
        found.map_or(&[], |(_, words)| words.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_skip_comments_and_split_at_the_last_bar() {
        let text = "# c\n\npostgres|COPY|A B\nhints|X Y\n";
        let all: Vec<_> = rows(text).collect();
        assert_eq!(all, [("postgres|COPY", "A B"), ("hints", "X Y")]);
        assert_eq!(WordLists::parse(text).get("hints"), ["X", "Y"]);
    }
}
