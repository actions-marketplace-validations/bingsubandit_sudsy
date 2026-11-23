//! The elements of a markup file as a flat tree: name, attributes, parent, children and whether
//! text stands directly in an element. Rules over the structure of a document (what an element may
//! contain, what a link or a form holds) read this instead of walking syntax nodes. An element is a
//! node whose first child node is a `#start-tag` with the slot `name` and the attribute nodes in
//! `entries` (each a `#key-value` with `key` and `value`); `#content` nodes that are not elements
//! (interpolations, blocks) count as text, other nodes are looked through.
//!
//! concept: markup-tree
//! what: The elements of a markup document with names, attributes, parents and children.
//! tools: html-validate element-permitted-content, wcag/h30, hidden-focusable
//! reads: tag:start-tag, tag:key-value, tag:chunk, tag:content

use core_syntax::{Grammar, SyntaxNode, TextRange, TextSize};

/// An attribute as written.
#[derive(Clone, Debug)]
pub struct Attr {
    pub name: String,
    /// Without quotes; `None` for an attribute without `=`.
    pub value: Option<String>,
    pub key: TextRange,
    pub value_range: Option<TextRange>,
}

#[derive(Clone, Debug)]
pub struct Element {
    pub name: String,
    pub name_range: TextRange,
    /// The start tag.
    pub range: TextRange,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub attrs: Vec<Attr>,
    /// Text that is not blank (or an interpolation) stands directly in the element.
    pub text: bool,
    /// The element has an end tag.
    pub end_tag: bool,
    /// The start tag ends in `/>`.
    pub empty: bool,
    /// Not written: added by a content model that opens elements implicitly.
    pub implied: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Markup {
    pub elements: Vec<Element>,
    /// Elements that have no parent element.
    pub roots: Vec<usize>,
}

impl Markup {
    /// The elements of `root` in document order. `fold` lowercases names (HTML).
    pub fn build(root: &SyntaxNode, grammar: &Grammar, text: &str, fold: bool) -> Markup {
        let mut markup = Markup::default();
        markup.content(root, None, grammar, text, fold);
        markup
    }

    /// The children of `node` (an element or the root): elements are added; other `#content`
    /// nodes and non-blank text tokens make `parent` hold text.
    fn content(
        &mut self,
        node: &SyntaxNode,
        parent: Option<usize>,
        grammar: &Grammar,
        text: &str,
        fold: bool,
    ) {
        for child in node.children_with_tokens() {
            match child {
                rowan::NodeOrToken::Token(t) => {
                    if let Some(p) = parent
                        && grammar.has_tag(t.kind(), "chunk")
                        && !text[t.text_range()].trim().is_empty()
                    {
                        self.elements[p].text = true;
                    }
                }
                rowan::NodeOrToken::Node(n) => {
                    if let Some(index) = self.element(&n, parent, grammar, text, fold) {
                        self.content(&n, Some(index), grammar, text, fold);
                        self.elements[index].end_tag = grammar
                            .slot(&n, "end")
                            .is_some_and(|e| e.as_node().is_some());
                    } else if grammar.has_tag(n.kind(), "content") {
                        if let Some(p) = parent {
                            self.elements[p].text = true;
                        }
                    } else {
                        self.content(&n, parent, grammar, text, fold);
                    }
                }
            }
        }
    }

    /// Adds the element `node` is, when it is one.
    fn element(
        &mut self,
        node: &SyntaxNode,
        parent: Option<usize>,
        grammar: &Grammar,
        text: &str,
        fold: bool,
    ) -> Option<usize> {
        let start = node.first_child()?;
        if !grammar.has_tag(start.kind(), "start-tag") {
            return None;
        }
        let name_range = grammar.slot(&start, "name")?.text_range();
        let raw = &text[name_range];
        let attrs = grammar
            .slots(&start, "entries")
            .filter_map(|e| e.into_node())
            .filter_map(|a| attribute(&a, grammar, text, fold))
            .collect();
        // The node may start with trivia: the tag starts at the `<` before its name.
        let from = name_range.start() - TextSize::from(1);
        let tag = text[TextRange::new(from, start.text_range().end())].trim_end();
        let index = self.elements.len();
        self.elements.push(Element {
            name: if fold {
                raw.to_ascii_lowercase()
            } else {
                raw.to_string()
            },
            name_range,
            range: TextRange::at(from, TextSize::of(tag)),
            parent,
            children: Vec::new(),
            attrs,
            text: false,
            end_tag: false,
            empty: tag.ends_with("/>"),
            implied: false,
        });
        match parent {
            Some(p) => self.elements[p].children.push(index),
            None => self.roots.push(index),
        }
        Some(index)
    }

    pub fn attr(&self, element: usize, name: &str) -> Option<&Attr> {
        self.elements[element].attrs.iter().find(|a| a.name == name)
    }

    /// The elements below `element`, in document order.
    pub fn descendants(&self, element: usize) -> Vec<usize> {
        let mut out = Vec::new();
        let mut stack: Vec<usize> = self.elements[element]
            .children
            .iter()
            .rev()
            .copied()
            .collect();
        while let Some(i) = stack.pop() {
            out.push(i);
            stack.extend(self.elements[i].children.iter().rev());
        }
        out
    }

    /// `element` and its ancestors, nearest first.
    pub fn ancestors_and_self(&self, element: usize) -> impl Iterator<Item = usize> + '_ {
        std::iter::successors(Some(element), |&i| self.elements[i].parent)
    }
}

fn attribute(node: &SyntaxNode, grammar: &Grammar, text: &str, fold: bool) -> Option<Attr> {
    let key = grammar.slot(node, "key")?.text_range();
    let value = grammar.slot(node, "value").map(|v| {
        let range = v.text_range();
        let raw = &text[range];
        let quoted = raw.len() >= 2
            && ((raw.starts_with('"') && raw.ends_with('"'))
                || (raw.starts_with('\'') && raw.ends_with('\'')));
        if quoted {
            let inner = TextRange::new(
                range.start() + TextSize::from(1),
                range.end() - TextSize::from(1),
            );
            (text[inner].to_string(), inner)
        } else {
            (raw.to_string(), range)
        }
    });
    let name = &text[key];
    Some(Attr {
        name: if fold {
            name.to_ascii_lowercase()
        } else {
            name.to_string()
        },
        value: value.as_ref().map(|v| v.0.clone()),
        key,
        value_range: value.map(|v| v.1),
    })
}
