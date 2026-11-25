//! The signature of a command: what a call may contain and how its words group. A row names the
//! command, its leading positional parameters and the mandatory ones, its keywords (each with the
//! kind of values that follow it, and for a keyword that opens a section the keywords inside),
//! and its forms: a command whose first words choose the signature (`list(APPEND ...)`,
//! `string(REGEX REPLACE ...)`). A language writes its rows as a static table, generated from
//! the documentation of the tool that owns the commands; rules and layouts read them.
//!
//! concept: command-signature
//! what: A table row per command: positional parameters, mandatory ones, keywords with their value kinds, nested sections and forms chosen by the first words.
//! tools: PSScriptAnalyzer PSUseCmdletCorrectly, gersemi keyword sections
//! reads: path:signature::Signature

/// What follows a keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Values {
    /// Nothing: the keyword is an option.
    None,
    /// One value.
    One,
    /// Any number of values.
    Many,
    /// Any number of `name value` pairs.
    Pairs,
    /// Any number of words that form a command line.
    CommandLine,
    /// An operator between two operands (`<a> AND <b>`): it starts a line of its own, with the
    /// operands that follow it up to the next operator; what stands in front of the first operator
    /// is one line.
    Infix,
}

/// A keyword of a call, with what follows it and, for a section, the keywords inside it.
#[derive(Debug, Clone, Copy)]
pub struct Keyword {
    pub word: &'static str,
    pub values: Values,
    pub inner: &'static [Keyword],
}

/// A signature chosen by the first words of the call.
#[derive(Debug, Clone, Copy)]
pub struct Form {
    /// The words that choose it (one or two), compared without case.
    pub words: &'static [&'static str],
    /// Positional arguments that follow the words and stay with them.
    pub positional: usize,
    pub keywords: &'static [Keyword],
}

/// The row of one command.
#[derive(Debug, Clone, Copy)]
pub struct Signature {
    pub name: &'static str,
    /// Its leading positional parameters, in order.
    pub positional: &'static [&'static str],
    /// The parameters it requires.
    pub mandatory: &'static [&'static str],
    pub keywords: &'static [Keyword],
    pub forms: &'static [Form],
}

impl Signature {
    /// A row with nothing but a name; the rows of a table fill in what they know.
    pub const NONE: Signature = Signature {
        name: "",
        positional: &[],
        mandatory: &[],
        keywords: &[],
        forms: &[],
    };
}

/// The row of the command `name` in `table` (sorted by lower-case name), compared without case.
pub fn lookup<'a>(table: &'a [Signature], name: &str) -> Option<&'a Signature> {
    let low = |s: &str| {
        s.bytes()
            .map(|b| b.to_ascii_lowercase())
            .collect::<Vec<_>>()
    };
    let key = low(name);
    let at = table
        .binary_search_by(|row| {
            row.name
                .bytes()
                .map(|b| b.to_ascii_lowercase())
                .cmp(key.iter().copied())
        })
        .ok()?;
    Some(&table[at])
}

/// The keyword `word` in `list`, compared without case.
pub fn find<'a>(list: &'a [Keyword], word: &str) -> Option<&'a Keyword> {
    list.iter().find(|k| k.word.eq_ignore_ascii_case(word))
}
