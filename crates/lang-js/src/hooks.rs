//! What the shared rules ask of JavaScript: the static name of an object key.
//!
//! concept: key-identity

use crate::rules::key_name;
use core_rules::RuleHooks;
use core_syntax::{Grammar, SyntaxElement};
use std::borrow::Cow;

pub struct JsHooks;

impl RuleHooks for JsHooks {
    fn key_name<'a>(&self, _g: &Grammar, key: &SyntaxElement, text: &'a str) -> Cow<'a, str> {
        match key {
            SyntaxElement::Node(n) => key_name(n, text).map_or(Cow::Borrowed(""), Cow::Owned),
            SyntaxElement::Token(_) => Cow::Borrowed(""),
        }
    }

    /// A computed key (`[a]`) has no static name.
    fn is_name(&self, key: &SyntaxElement, text: &str) -> bool {
        matches!(key, SyntaxElement::Node(n) if key_name(n, text).is_some())
    }
}
