//! Declaration rows: which declarations a rule of a family looks at. A row is a grammar tag,
//! optionally `<parent-tag` (the parent has that tag: the node with the modifiers; `<a<b`: the
//! parent has tag `a` and the grandparent `b`), then `&word` for each modifier keyword the
//! declaration (the parent, with a parent tag) must have, `&@name` for each annotation it must
//! carry (`*` matches any run of characters, so `&@*Test` is `@Test` and `@ParameterizedTest`)
//! and `&+slot` for a slot (a field of the grammar) the declaration must have, then `=path` (a [`SlotPath`] to the name; the node's
//! `name` slot, or its `key` slot, by default). Rows are `|`-separated.
//!
//! concept: declaration-rows
//! what: Rows naming declarations by grammar tag, modifier keywords and annotations.
//! tools: Checkstyle name checks, graphql-eslint naming-convention
//! reads: tag:modifiers, tag:annotation

use crate::slot_path::SlotPath;
use core_syntax::{Grammar, SyntaxNode};

pub struct Row {
    pub option: String,
    pub tag: String,
    /// Tags of the parent, the grandparent and so on.
    pub parents: Vec<String>,
    pub modifiers: Vec<String>,
    pub annotations: Vec<String>,
    pub slots: Vec<String>,
    pub path: Option<SlotPath>,
}

impl Row {
    pub fn parse(option: &str, alt: &str) -> Result<Row, String> {
        let (alt, path) = match alt.split_once('=') {
            Some((a, p)) => (a, Some(SlotPath::parse(p)?)),
            None => (alt, None),
        };
        let mut parts = alt.split('&');
        let head = parts.next().unwrap_or_default();
        let mut chain = head.split('<');
        let tag = chain.next().unwrap_or_default();
        let parents: Vec<String> = chain.map(str::to_string).collect();
        let (annotations, rest): (Vec<&str>, Vec<&str>) = parts.partition(|p| p.starts_with('@'));
        let (slots, modifiers): (Vec<&str>, Vec<&str>) =
            rest.into_iter().partition(|p| p.starts_with('+'));
        Ok(Row {
            option: option.to_string(),
            tag: tag.to_string(),
            parents,
            modifiers: modifiers.into_iter().map(str::to_string).collect(),
            annotations: annotations
                .into_iter()
                .map(|a| a[1..].to_string())
                .collect(),
            slots: slots.into_iter().map(|s| s[1..].to_string()).collect(),
            path,
        })
    }

    /// How specific the row is: rows with more conditions are tried first (a modifier or annotation
    /// counts twice: a `const` declaration is more specific than one inside an object).
    pub fn conditions(&self) -> usize {
        self.parents.len() + 2 * (self.modifiers.len() + self.annotations.len()) + self.slots.len()
    }

    pub fn matches(&self, g: &Grammar, node: &SyntaxNode) -> bool {
        if !g.has_tag(node.kind(), &self.tag) {
            return false;
        }
        let mut cursor = node.clone();
        let mut holder = node.clone();
        for (i, tag) in self.parents.iter().enumerate() {
            let Some(up) = cursor.parent().filter(|p| g.has_tag(p.kind(), tag)) else {
                return false;
            };
            if i == 0 {
                holder = up.clone();
            }
            cursor = up;
        }
        if !self.slots.iter().all(|s| g.slot(node, s).is_some()) {
            return false;
        }
        self.modifiers.iter().all(|m| has_modifier(g, &holder, m))
            && self
                .annotations
                .iter()
                .all(|a| annotation_names(g, &holder).iter().any(|n| glob(a, n)))
    }
}

/// Whether `node` has a `modifiers` child with the keyword `word`.
pub fn has_modifier(g: &Grammar, node: &SyntaxNode, word: &str) -> bool {
    node.children()
        .filter(|c| g.has_tag(c.kind(), "modifiers"))
        .any(|m| {
            m.children_with_tokens()
                .filter_map(|e| e.into_token())
                .any(|t| t.text() == word)
        })
}

/// The names of the annotations of `node`'s `modifiers` (`Test` for `@Test`, `org.junit.Test`
/// for `@org.junit.Test(x)`): the dotted name after the `@`.
pub fn annotation_names(g: &Grammar, node: &SyntaxNode) -> Vec<String> {
    let mut names = Vec::new();
    for m in node.children().filter(|c| g.has_tag(c.kind(), "modifiers")) {
        for a in m.children().filter(|c| g.has_tag(c.kind(), "annotation")) {
            let mut name = String::new();
            let tokens = a
                .descendants_with_tokens()
                .filter_map(|e| e.into_token())
                .filter(|t| !g.is_trivia(t.kind()));
            for t in tokens.skip_while(|t| t.text() != "@").skip(1) {
                if t.text() == "(" {
                    break;
                }
                name.push_str(t.text());
            }
            names.push(name);
        }
    }
    names
}

/// Whether `text` matches `pattern`, in which `*` stands for any run of characters.
pub fn glob(pattern: &str, text: &str) -> bool {
    let mut parts = pattern.split('*');
    let first = parts.next().unwrap_or_default();
    let Some(mut rest) = text.strip_prefix(first) else {
        return false;
    };
    let mut parts: Vec<&str> = parts.collect();
    let Some(last) = parts.pop() else {
        return rest.is_empty();
    };
    for part in parts {
        match rest.find(part) {
            Some(at) => rest = &rest[at + part.len()..],
            None => return false,
        }
    }
    rest.ends_with(last)
}
