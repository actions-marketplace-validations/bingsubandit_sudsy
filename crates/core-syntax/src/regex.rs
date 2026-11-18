//! The `regex` and `regex-call` rows of a grammar: which tokens hold regex source and which calls
//! take it as their first argument, with the name of the regex flavor (the dialect; its facts are
//! in `core_rules::regex_source`).

use crate::kind::SyntaxKind;

/// A token kind whose text is a regex literal: `quote`, the pattern, `quote`, then flags.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegexRow {
    pub token: String,
    pub kind: Option<SyntaxKind>,
    pub flavor: String,
    pub quote: char,
}

impl RegexRow {
    /// The pattern of a literal's text (`/a+/g` is `a+`).
    pub fn pattern<'a>(&self, text: &'a str) -> Option<&'a str> {
        let body = text.strip_prefix(self.quote)?;
        Some(&body[..body.rfind(self.quote)?])
    }
}

/// Calls whose first argument is a regex source.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RegexCall {
    pub flavor: String,
    /// The operator that joins two strings into one; none: the argument is one literal.
    pub concat: Option<String>,
    /// The text of the callee, without spaces (`RegExp`, `re.compile`).
    pub callees: Vec<String>,
}
