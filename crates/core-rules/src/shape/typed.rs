//! Entries that declare a type, a default and options (the inputs of a workflow): the default
//! must fit the type and the options, options belong to the choice type only. The language gives
//! the key names and the types; the checks are here.
//!
//! concept: none (infrastructure: shape typed entries)

use super::{Val, Walk};
use crate::shape::Row;
use core_syntax::{SyntaxNode, TextRange};

#[derive(Clone, Copy, PartialEq)]
pub enum Value {
    Bool,
    Number,
}

pub struct Typed {
    pub type_key: &'static str,
    pub default_key: &'static str,
    pub options_key: &'static str,
    pub required_key: &'static str,
    /// The type that has options; empty: no type has.
    pub choice: &'static str,
    /// The types whose default must parse as a value.
    pub types: &'static [(&'static str, Value)],
    /// A required entry must not have a default.
    pub required_no_default: bool,
}

impl Walk<'_> {
    /// Checks each entry (a mapping) of the mapping that `entries` came from.
    pub(super) fn typed(
        &mut self,
        row: &'static Row,
        typed: &Typed,
        entries: &[(String, TextRange, Option<SyntaxNode>)],
    ) {
        for (name, key, value) in entries {
            let Val::Map(map) = self.val(value.as_ref()) else {
                continue;
            };
            let fields = self.entries(&map);
            let find = |k: &str| fields.iter().find(|f| f.0 == k);
            let scalar = |k: &str| match find(k).map(|f| self.val(f.2.as_ref())) {
                Some(Val::Scalar { text, .. }) => Some(text),
                _ => None,
            };
            let ty = scalar(typed.type_key).unwrap_or_default();
            let required = scalar(typed.required_key);
            let has_choice = !typed.choice.is_empty() && ty == typed.choice;
            let mut options: Vec<(String, TextRange)> = Vec::new();
            if let Some((_, _, Some(node))) = find(typed.options_key)
                && let Val::Seq(seq) = self.val(Some(node))
            {
                for (range, item) in self.items(&seq) {
                    if let Val::Scalar { text, .. } = self.val(item.as_ref()) {
                        options.push((text, range));
                    }
                }
            }
            if has_choice && options.is_empty() {
                let message = format!(
                    "input type of \"{name}\" is \"{ty}\" but \"{}\" is not set",
                    typed.options_key
                );
                self.report(row, *key, message);
            }
            if find(typed.options_key).is_some() && !has_choice {
                let message = format!(
                    "\"{}\" can not be set to \"{name}\" input because its input type is not \"{}\"",
                    typed.options_key, typed.choice
                );
                self.report(row, *key, message);
            }
            for (i, (text, range)) in options.iter().enumerate() {
                if options[..i].iter().any(|(t, _)| t == text) {
                    let message =
                        format!("option \"{text}\" is duplicated in options of \"{name}\" input");
                    self.report(row, *range, message);
                }
            }
            let Some((_, drange, dnode)) = find(typed.default_key) else {
                continue;
            };
            let Val::Scalar { text, .. } = self.val(dnode.as_ref()) else {
                continue;
            };
            if self.is_expression(&text) {
                continue;
            }
            let problem = if has_choice
                && !options.is_empty()
                && !options.iter().any(|(o, _)| *o == text)
            {
                Some(format!(
                    "default value \"{text}\" of \"{name}\" input is not included in its options"
                ))
            } else {
                match typed.types.iter().find(|(t, _)| *t == ty).map(|t| t.1) {
                    Some(Value::Bool) if text != "true" && text != "false" => Some(format!(
                        "type of \"{name}\" input is \"{ty}\". its default value \"{text}\" must be \"true\" or \"false\""
                    )),
                    Some(Value::Number) if text.parse::<f64>().is_err() => Some(format!(
                        "type of \"{name}\" input is \"{ty}\" but its default value \"{text}\" cannot be parsed as a float number"
                    )),
                    _ => None,
                }
            };
            if let Some(message) = problem {
                self.report(row, *drange, message);
            }
            if typed.required_no_default && required.as_deref() == Some("true") {
                let message = format!(
                    "input \"{name}\" has the default value \"{text}\", but it is also required. if an input is marked as required, its default value will never be used"
                );
                self.report(row, *drange, message);
            }
        }
    }
}
