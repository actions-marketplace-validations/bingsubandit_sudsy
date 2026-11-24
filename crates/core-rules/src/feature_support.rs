//! Is a markup feature (an element, an attribute, an attribute value) available in browsers at a
//! given level? The language passes a generated table of rows `key status year` (status 10: widely
//! available, 5: newly available, 0: not available in all browsers; year of the baseline, empty
//! for 0). Keys are lowercase: `element`, `element.attribute`, `element.attribute.value`, and
//! `@attribute`, `@attribute.value` for a global attribute. A name the table does not list is
//! available. The table is parsed on first use.
//!
//! concept: feature-support
//! what: An element, attribute or attribute value that browsers do not all support at the wanted level (widely, newly or since a year).
//! tools: html-eslint use-baseline
//! reads: rule:use-baseline

use core_syntax::TextRange;
use std::collections::HashMap;
use std::sync::OnceLock;

type Rows = HashMap<&'static str, (u8, u16)>;

pub struct SupportTable {
    text: &'static str,
    rows: OnceLock<Rows>,
}

/// The level of the `available` option: `widely`, `newly` or a year (the feature is a baseline
/// feature of that year or before). Other text means `widely`.
#[derive(Clone, Copy)]
pub enum Availability {
    Widely,
    Newly,
    Year(u16),
}

/// What a check found, in the order of its look-ups.
#[derive(Debug, PartialEq, Eq)]
pub enum Unsupported {
    /// The attribute of the element (`global`: of any element).
    Attribute { global: bool },
    /// The value of the attribute.
    Value { global: bool },
}

impl Availability {
    pub fn parse(text: &str) -> Availability {
        match text.trim() {
            "newly" => Availability::Newly,
            t => t.parse().map_or(Availability::Widely, Availability::Year),
        }
    }
}

impl SupportTable {
    pub const fn new(text: &'static str) -> SupportTable {
        SupportTable {
            text,
            rows: OnceLock::new(),
        }
    }

    fn rows(&self) -> &Rows {
        self.rows.get_or_init(|| {
            let mut rows = Rows::new();
            for line in self.text.lines().filter(|l| !l.starts_with('#')) {
                let mut f = line.split_whitespace();
                if let (Some(key), Some(status)) = (f.next(), f.next()) {
                    let year = f.next().and_then(|y| y.parse().ok()).unwrap_or(0);
                    rows.insert(key, (status.parse().unwrap_or(0), year));
                }
            }
            rows
        })
    }

    /// Whether the key written by joining `parts` (lowercased) is available at `level`; a key
    /// the table does not list, or one too long to be a name, is. No allocation.
    fn holds(&self, level: Availability, parts: &[&str]) -> bool {
        let mut buf = [0u8; 128];
        let mut len = 0;
        for byte in parts.iter().flat_map(|p| p.bytes()) {
            if len == buf.len() {
                return true;
            }
            buf[len] = byte.to_ascii_lowercase();
            len += 1;
        }
        let Ok(key) = std::str::from_utf8(&buf[..len]) else {
            return true;
        };
        match self.rows().get(key) {
            None => true,
            Some(&(status, year)) => match level {
                Availability::Widely => status >= 10,
                Availability::Newly => status >= 5,
                Availability::Year(y) => status > 0 && year <= y,
            },
        }
    }

    /// Is the element available? `name` is lowercase.
    pub fn element(&self, level: Availability, name: &str) -> bool {
        self.holds(level, &[name])
    }

    /// The first look-up that fails for an attribute (and its value, when it has one) of an
    /// element: the attribute of the element, the global attribute, the value for the element,
    /// the value as a global attribute's. `element` and `attr` are lowercase; the value is
    /// trimmed and lowercased here.
    pub fn attribute(
        &self,
        level: Availability,
        element: &str,
        attr: &str,
        value: Option<&str>,
    ) -> Option<Unsupported> {
        if !self.holds(level, &[element, ".", attr]) {
            return Some(Unsupported::Attribute { global: false });
        }
        if !self.holds(level, &["@", attr]) {
            return Some(Unsupported::Attribute { global: true });
        }
        let value = value?.trim();
        if !self.holds(level, &[element, ".", attr, ".", value]) {
            Some(Unsupported::Value { global: false })
        } else if !self.holds(level, &["@", attr, ".", value]) {
            Some(Unsupported::Value { global: true })
        } else {
            None
        }
    }
}

/// An attribute of the element being checked, with the ranges to report at.
pub struct AttrRef<'a> {
    pub name: &'a str,
    pub name_range: TextRange,
    pub value: Option<&'a str>,
    pub value_range: Option<TextRange>,
}

impl SupportTable {
    /// The unsupported features of one element, as (range, text): `Element '<x>'` at
    /// `element_range` (the attributes are then left out), or `Attribute 'a' on '<x>'`,
    /// `Attribute 'a'` (global) and `Attribute 'a="v"'` forms at the attribute's name or value.
    /// A custom element (a `-` in the name), an attribute in `skip` and one with a template
    /// expression (`{`) are not judged.
    pub fn features<'a>(
        &self,
        level: Availability,
        element: &str,
        element_range: TextRange,
        attrs: impl IntoIterator<Item = AttrRef<'a>>,
        skip: &[String],
    ) -> Vec<(TextRange, String)> {
        if element.contains('-') {
            return Vec::new();
        }
        if !self.element(level, element) {
            return vec![(element_range, format!("Element '<{element}>'"))];
        }
        let mut found = Vec::new();
        for a in attrs {
            if a.name.contains('{') || skip.iter().any(|s| s == a.name) {
                continue;
            }
            let Some(kind) = self.attribute(level, element, a.name, a.value) else {
                continue;
            };
            let on = |global: bool| {
                if global {
                    String::new()
                } else {
                    format!(" on '<{element}>'")
                }
            };
            found.push(match kind {
                Unsupported::Attribute { global } => (
                    a.name_range,
                    format!("Attribute '{}'{}", a.name, on(global)),
                ),
                Unsupported::Value { global } => (
                    a.value_range.unwrap_or(a.name_range),
                    format!(
                        "Attribute '{}=\"{}\"'{}",
                        a.name,
                        a.value.unwrap_or(""),
                        on(global)
                    ),
                ),
            });
        }
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static T: SupportTable = SupportTable::new(
        "a 10 2015\ndatalist 5 2025\ndialog 0\n@popover 10 2024\n@popover.hint 0\nlink.rel.manifest 0\n",
    );

    #[test]
    fn levels() {
        let w = Availability::Widely;
        assert!(T.element(w, "a") && !T.element(w, "datalist") && !T.element(w, "dialog"));
        assert!(T.element(w, "nothing"));
        assert!(T.element(Availability::Newly, "datalist"));
        assert!(!T.element(Availability::Newly, "dialog"));
        assert!(T.element(Availability::Year(2025), "datalist"));
        assert!(!T.element(Availability::Year(2024), "datalist"));
    }

    #[test]
    fn attributes() {
        let w = Availability::Widely;
        assert_eq!(T.attribute(w, "div", "popover", Some("auto")), None);
        assert_eq!(
            T.attribute(w, "div", "popover", Some(" HINT ")),
            Some(Unsupported::Value { global: true })
        );
        assert_eq!(
            T.attribute(w, "link", "rel", Some("manifest")),
            Some(Unsupported::Value { global: false })
        );
        assert_eq!(T.attribute(w, "link", "rel", Some("unknown")), None);
        assert_eq!(T.attribute(w, "link", "unknown", None), None);
    }
}
