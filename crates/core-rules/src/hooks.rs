//! What the shared rules ask a language about its own syntax. A language that tags its grammar
//! (`#mapping`, `#sequence`, `#key-part`) and gives it a `keys` line gets the shared key and
//! sequence rules with no code of its own; a language whose keys or scalars the rows cannot
//! describe implements the hooks it needs.
//!
//! concept: key-identity
//! what: What a key or scalar is called, read from tags and the keys row, for the key rules.
//! tools: eslint-plugin-jsonc, yamllint key rules
//! reads: grammar:keys

use crate::names;
use crate::rule::FileContext;
use core_syntax::{Grammar, SyntaxElement, SyntaxNode, TextRange};
use std::borrow::Cow;

/// A scalar the sorting of sequence items understands.
#[derive(Clone, Debug, PartialEq)]
pub enum Scalar<'a> {
    String(Cow<'a, str>),
    Number(f64),
}

/// What a language's words are, for a word that is written as a name: a plain word, a keyword the
/// language lets one use as a name, or a reserved word it does not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Word {
    Plain,
    Keyword,
    Reserved,
}

impl Word {
    /// The class of `upper` in `version`: reserved wins over keyword.
    pub fn of(hooks: &dyn RuleHooks, version: Option<&str>, upper: &str) -> Word {
        if hooks.is_reserved(version, upper) {
            Word::Reserved
        } else if hooks.is_keyword(version, upper) {
            Word::Keyword
        } else {
            Word::Plain
        }
    }
}

/// The case an unquoted name is folded to (`Fold::None`: kept as written).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fold {
    Upper,
    Lower,
    None,
}

/// A getter or setter entry of an object: one of each may share a name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Accessor {
    Get,
    Set,
}

pub trait RuleHooks: Send + Sync {
    /// The keys, in order, of a `#mapping` whose entries are not nodes of their own (a Python
    /// dict is flat: `key : value , ...`); `None` when the entries are `#key-value` nodes.
    fn flat_keys(&self, _mapping: &SyntaxNode) -> Option<Vec<SyntaxElement>> {
        None
    }

    /// Whether a key is a name at all: `no-duplicate-keys` skips the others (HTML: template
    /// words inside a start tag, `{{ if }}`). The default is every key, or with `keys static` the
    /// keys that have a static name ([`names::is_name`]).
    fn is_name(&self, g: &Grammar, key: &SyntaxElement, _text: &str) -> bool {
        names::is_name(g, key)
    }

    /// The identity of a key (the `key` item of a `#key-value` entry): the name that is compared,
    /// sorted and checked. `text` is the file's text. The default reads the grammar's `keys` line
    /// ([`names::key_name`]).
    fn key_name<'a>(&self, g: &Grammar, key: &SyntaxElement, text: &'a str) -> Cow<'a, str> {
        names::key_name(g, key, text)
    }

    /// Whether a token kind is an anchor (`Some(true)`), an alias (`Some(false)`) or neither.
    fn reference(&self, _kind: core_syntax::SyntaxKind) -> Option<bool> {
        None
    }

    /// The bytes a reference starts with (YAML `&` and `*`: an anchor and an alias); at most two.
    /// The reference rules look only at these bytes of the text. Default: no references.
    fn reference_marks(&self) -> &'static [u8] {
        &[]
    }

    /// The value of a string or number item of a sequence (`item` is the value node); `None` for
    /// anything else, which `sorted-array-values` leaves alone ([`names::scalar`]).
    fn scalar<'a>(&self, g: &Grammar, item: &SyntaxNode, text: &'a str) -> Option<Scalar<'a>> {
        names::scalar(g, item, text)
    }

    /// Is the line a `key: value` entry whose value is one unbreakable word (no space)? Lets
    /// `max-line-length` accept it with `allow_non_breakable_inline_mappings`. The default has no
    /// such lines.
    /// The language's `format.rules` text (`NodeFormatter::rows`): a rule with a section named like
    /// it there reads its rows from it, in lint and when the formatter runs it.
    fn rows(&self) -> &'static str {
        ""
    }

    fn inline_mapping_line(&self, _line: &str) -> bool {
        false
    }

    /// The dialect or language version the file is read in (`dialect` for SQL), when it has one.
    fn language_version(&self) -> Option<&str> {
        None
    }

    /// Is `upper` (upper case) a reserved word of the dialect `version` (`None`: the default one)?
    /// The default has none.
    fn is_reserved(&self, _version: Option<&str>, _upper: &str) -> bool {
        false
    }

    /// Is `upper` (upper case) a keyword of the dialect `version`, reserved or not?
    fn is_keyword(&self, _version: Option<&str>, _upper: &str) -> bool {
        false
    }

    /// The case the dialect `version` folds unquoted names to.
    fn fold(&self, _version: Option<&str>) -> Fold {
        Fold::None
    }

    /// Rules the code silences for a range by its own syntax (PowerShell's
    /// `SuppressMessageAttribute`), as `(rule id, range)`. The default has none.
    fn suppressed_ranges(&self, _file: &FileContext<'_>) -> Vec<(String, TextRange)> {
        Vec::new()
    }
}

/// `inner` with the language's rows (`RuleHooks::rows`): what a provider hands the rules when its
/// formatter has rows.
pub struct WithRows {
    pub inner: &'static dyn RuleHooks,
    pub rows: &'static str,
}

impl RuleHooks for WithRows {
    fn flat_keys(&self, mapping: &SyntaxNode) -> Option<Vec<SyntaxElement>> {
        self.inner.flat_keys(mapping)
    }

    fn is_name(&self, g: &Grammar, key: &SyntaxElement, text: &str) -> bool {
        self.inner.is_name(g, key, text)
    }

    fn key_name<'a>(&self, g: &Grammar, key: &SyntaxElement, text: &'a str) -> Cow<'a, str> {
        self.inner.key_name(g, key, text)
    }

    fn scalar<'a>(&self, g: &Grammar, item: &SyntaxNode, text: &'a str) -> Option<Scalar<'a>> {
        self.inner.scalar(g, item, text)
    }

    fn rows(&self) -> &'static str {
        self.rows
    }

    fn inline_mapping_line(&self, line: &str) -> bool {
        self.inner.inline_mapping_line(line)
    }

    fn language_version(&self) -> Option<&str> {
        self.inner.language_version()
    }

    fn is_reserved(&self, version: Option<&str>, upper: &str) -> bool {
        self.inner.is_reserved(version, upper)
    }

    fn is_keyword(&self, version: Option<&str>, upper: &str) -> bool {
        self.inner.is_keyword(version, upper)
    }

    fn fold(&self, version: Option<&str>) -> Fold {
        self.inner.fold(version)
    }

    fn suppressed_ranges(&self, file: &FileContext<'_>) -> Vec<(String, TextRange)> {
        self.inner.suppressed_ranges(file)
    }
}

/// `inner` for one file read in the dialect or version `version`.
pub struct WithVersion<'a> {
    pub inner: &'a dyn RuleHooks,
    pub version: Option<&'a str>,
}

impl RuleHooks for WithVersion<'_> {
    fn flat_keys(&self, mapping: &SyntaxNode) -> Option<Vec<SyntaxElement>> {
        self.inner.flat_keys(mapping)
    }

    fn is_name(&self, g: &Grammar, key: &SyntaxElement, text: &str) -> bool {
        self.inner.is_name(g, key, text)
    }

    fn key_name<'a>(&self, g: &Grammar, key: &SyntaxElement, text: &'a str) -> Cow<'a, str> {
        self.inner.key_name(g, key, text)
    }

    fn scalar<'a>(&self, g: &Grammar, item: &SyntaxNode, text: &'a str) -> Option<Scalar<'a>> {
        self.inner.scalar(g, item, text)
    }

    fn rows(&self) -> &'static str {
        self.inner.rows()
    }

    fn inline_mapping_line(&self, line: &str) -> bool {
        self.inner.inline_mapping_line(line)
    }

    fn language_version(&self) -> Option<&str> {
        self.version
    }

    fn is_reserved(&self, version: Option<&str>, upper: &str) -> bool {
        self.inner.is_reserved(version, upper)
    }

    fn is_keyword(&self, version: Option<&str>, upper: &str) -> bool {
        self.inner.is_keyword(version, upper)
    }

    fn fold(&self, version: Option<&str>) -> Fold {
        self.inner.fold(version)
    }

    fn suppressed_ranges(&self, file: &FileContext<'_>) -> Vec<(String, TextRange)> {
        self.inner.suppressed_ranges(file)
    }
}

/// The hooks of a language that has none of its own.
pub struct DefaultHooks;

impl RuleHooks for DefaultHooks {}

pub static DEFAULT_HOOKS: DefaultHooks = DefaultHooks;
