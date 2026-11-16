//! The `template` family: checks of Vue template syntax on attributes and tags. They are Rust
//! rather than pattern files because a pattern rule compiles a regex that costs more than all
//! of these rules together (the lint bench of `.vue` and `.svelte` inputs).
//!
//! Param `check` (default `directive`; the others are named in [`Kind`]) and, for every check,
//! `vue-only` (the rule runs on `.vue` files only: in HTML, `slot`, `is` and `<template>` are
//! standard). `directive` reads the attributes called `name` (`v-if`, `v-bind`, ...; `:x`, `@x`
//! and `#x` are `v-bind`, `v-on` and `v-slot`, a plain attribute is `slot`) and reports one
//! when any of its conditions holds:
//! - `value`: `required`, `forbidden`, `unless-arg` / `unless-modifier` (required when there is
//!   no argument / no modifier), `iteration` (`item in items`), `vue-prefixed` (`vue:name`);
//! - `no-arg`; `modifiers` (`none`, or the comma-separated ones allowed); `banned` (modifiers
//!   that are reported, comma-separated); `numeric-modifier`;
//! - `with`: the element also has one of these directives; `after`: the previous element does
//!   not have one of them (`v-else` after `v-if`);
//! - `bad-host`: `html` (a plain lowercase element: not `<template>`, `<component>` or a
//!   component), `file-input` (an `<input type="file">`);
//! - `host`: the rule only looks at this element, `template`, `html` or `not-slot`.

use crate::generated::kind as k;
use core_rules::{Edit, Family, Params, Rule, RuleContext, RuleMeta};
use core_syntax::{SyntaxKind, SyntaxNode, SyntaxNodeExt, TextRange, TextSize};

pub const FAMILY: Family = Family {
    name: "template",
    build,
};

#[derive(Clone, Copy)]
enum Kind {
    Directive,
    Shorthand,
    Hyphenation,
    KeyOnChild,
    ChildContent,
    LoneTemplate,
    TextareaMustache,
    ComponentIs,
}

#[derive(Clone, Copy, PartialEq)]
enum Value {
    Any,
    Required,
    Forbidden,
    UnlessArg,
    UnlessModifier,
    Iteration,
    VuePrefixed,
}

#[derive(Clone, Copy, PartialEq)]
enum Host {
    Template,
    Html,
    NotSlot,
    FileInput,
}

enum Modifiers {
    Any,
    None,
    Only(Vec<String>),
}

struct Directive {
    name: String,
    /// The starts of the attribute texts that can be this directive.
    prefixes: Vec<String>,
    value: Value,
    no_arg: bool,
    modifiers: Modifiers,
    banned: Vec<String>,
    numeric: bool,
    with: Vec<String>,
    after: Vec<String>,
    host: Option<Host>,
    bad_host: Option<Host>,
    /// No condition at all: the attribute is reported wherever it is (a deprecated one).
    unconditional: bool,
}

enum Check {
    Directive(Box<Directive>),
    Shorthand { from: String, to: String },
    Hyphenation,
    KeyOnChild,
    ChildContent,
    LoneTemplate,
    TextareaMustache,
    ComponentIs,
}

struct Template {
    meta: &'static RuleMeta,
    check: Check,
    message: String,
    vue_only: bool,
}

fn list(text: Option<String>) -> Vec<String> {
    text.map_or_else(Vec::new, |t| {
        t.split(',').map(|s| s.trim().to_string()).collect()
    })
}

fn host(p: &mut Params, key: &str) -> Result<Option<Host>, String> {
    let Some(text) = p.optional(key) else {
        return Ok(None);
    };
    match text.as_str() {
        "template" => Ok(Some(Host::Template)),
        "html" => Ok(Some(Host::Html)),
        "not-slot" => Ok(Some(Host::NotSlot)),
        "file-input" => Ok(Some(Host::FileInput)),
        _ => Err(format!("param `{key}`: `{text}` is not a host")),
    }
}

impl Directive {
    fn build(p: &mut Params) -> Result<Directive, String> {
        let name = p.text("name")?;
        let prefixes = match name.as_str() {
            "v-bind" => vec!["v-bind".into(), ":".into()],
            "v-on" => vec!["v-on".into(), "@".into()],
            "v-slot" => vec!["v-slot".into(), "#".into()],
            _ => vec![name.clone()],
        };
        let value = p.choice(
            "value",
            false,
            &[
                ("any", Value::Any),
                ("required", Value::Required),
                ("forbidden", Value::Forbidden),
                ("unless-arg", Value::UnlessArg),
                ("unless-modifier", Value::UnlessModifier),
                ("iteration", Value::Iteration),
                ("vue-prefixed", Value::VuePrefixed),
            ],
        )?;
        let modifiers = match p.optional("modifiers").as_deref() {
            None => Modifiers::Any,
            Some("none") => Modifiers::None,
            Some(list) => Modifiers::Only(list.split(',').map(|s| s.trim().to_string()).collect()),
        };
        let mut d = Directive {
            name,
            prefixes,
            value,
            no_arg: p.flag("no-arg")?,
            modifiers,
            banned: list(p.optional("banned")),
            numeric: p.flag("numeric-modifier")?,
            with: list(p.optional("with")),
            after: list(p.optional("after")),
            host: host(p, "host")?,
            bad_host: host(p, "bad-host")?,
            unconditional: false,
        };
        d.unconditional = d.value == Value::Any
            && matches!(d.modifiers, Modifiers::Any)
            && !d.no_arg
            && d.banned.is_empty()
            && !d.numeric
            && d.with.is_empty()
            && d.after.is_empty()
            && d.bad_host.is_none();
        Ok(d)
    }
}

fn build(meta: &'static RuleMeta, mut p: Params) -> Result<Box<dyn Rule>, String> {
    let kind = p.choice(
        "check",
        false,
        &[
            ("directive", Kind::Directive),
            ("shorthand", Kind::Shorthand),
            ("hyphenation", Kind::Hyphenation),
            ("key-on-child", Kind::KeyOnChild),
            ("child-content", Kind::ChildContent),
            ("lone-template", Kind::LoneTemplate),
            ("textarea-mustache", Kind::TextareaMustache),
            ("component-is", Kind::ComponentIs),
        ],
    )?;
    let check = match kind {
        Kind::Directive => Check::Directive(Box::new(Directive::build(&mut p)?)),
        Kind::Shorthand => Check::Shorthand {
            from: p.text("from")?,
            to: p.text("to")?,
        },
        Kind::Hyphenation => Check::Hyphenation,
        Kind::KeyOnChild => Check::KeyOnChild,
        Kind::ChildContent => Check::ChildContent,
        Kind::LoneTemplate => Check::LoneTemplate,
        Kind::TextareaMustache => Check::TextareaMustache,
        Kind::ComponentIs => Check::ComponentIs,
    };
    let rule = Template {
        meta,
        check,
        message: p.text("message")?,
        vue_only: p.flag("vue-only")?,
    };
    p.done()?;
    Ok(Box::new(rule))
}

/// The text of an attribute node without the blank space before it (a line break and the
/// indentation belong to the attribute that follows), and where it starts.
fn code<'a>(node: &SyntaxNode, text: &'a str) -> (&'a str, TextSize) {
    let range = node.text_range();
    let s = &text[range];
    let trimmed = s.trim_start();
    (
        trimmed,
        range.start() + TextSize::of(&s[..s.len() - trimmed.len()]),
    )
}

/// An attribute: its name as written, split into what Vue means by it.
struct Attr<'a> {
    start: TextSize,
    full: &'a str,
    /// `v-if`, `v-bind` (also for `:x`), ... or a plain attribute's name.
    name: &'a str,
    directive: bool,
    /// What follows the name's `:` (or the shorthand character), `arg.mod.mod`.
    rest: &'a str,
    /// The text after the name: ` = "value"` and the trivia.
    after: &'a str,
}

impl<'a> Attr<'a> {
    fn parse(node: &SyntaxNode, text: &'a str) -> Option<Attr<'a>> {
        let (s, start) = code(node, text);
        let end = s
            .find(|c: char| c.is_ascii_whitespace() || c == '=')
            .unwrap_or(s.len());
        let full = s.get(..end).filter(|f| !f.is_empty())?;
        let (name, rest, directive) = if full.starts_with("v-") {
            match full.find([':', '.']) {
                Some(i) => (
                    &full[..i],
                    full[i..].strip_prefix(':').unwrap_or(&full[i..]),
                    true,
                ),
                None => (full, "", true),
            }
        } else {
            match full.as_bytes()[0] {
                b':' => ("v-bind", &full[1..], true),
                b'@' => ("v-on", &full[1..], true),
                b'#' => ("v-slot", &full[1..], true),
                _ => (full, "", false),
            }
        };
        Some(Attr {
            start,
            full,
            name,
            directive,
            rest,
            after: &s[end..],
        })
    }

    fn range(&self) -> TextRange {
        TextRange::at(self.start, TextSize::of(self.full))
    }

    /// The argument and the modifiers.
    fn parts(&self) -> (Option<&'a str>, impl Iterator<Item = &'a str>) {
        let rest = self.rest;
        let split = if rest.starts_with('[') {
            rest.find(']').map_or(rest.len(), |i| i + 1)
        } else {
            rest.find('.').unwrap_or(rest.len())
        };
        let (arg, mods) = rest.split_at(split);
        (
            (!arg.is_empty()).then_some(arg),
            mods.split('.').filter(|m| !m.is_empty()),
        )
    }

    /// The value without its quotes; `None` without `=`.
    fn value(&self) -> Option<&'a str> {
        let v = self.after.trim_start().strip_prefix('=')?.trim_start();
        Some(match v.chars().next() {
            Some(q @ ('"' | '\'')) => {
                let inner = &v[1..];
                &inner[..inner.find(q).unwrap_or(inner.len())]
            }
            _ => &v[..v.find(|c: char| c.is_ascii_whitespace()).unwrap_or(v.len())],
        })
    }
}

fn tag_name<'a>(tag: &SyntaxNode, text: &'a str) -> Option<(&'a str, TextRange)> {
    let t = tag.token(k::TAG_NAME)?;
    Some((&text[t.text_range()], t.text_range()))
}

/// The attributes of a start tag.
fn attrs<'a>(tag: &SyntaxNode, text: &'a str) -> Vec<Attr<'a>> {
    tag.children()
        .filter(|c| c.kind() == k::ATTRIBUTE)
        .filter_map(|c| Attr::parse(&c, text))
        .collect()
}

fn has(attrs: &[Attr<'_>], names: &[String]) -> bool {
    attrs.iter().any(|a| names.iter().any(|n| n == a.name))
}

/// A component: a tag in PascalCase, or with a `-` or `.`.
fn is_component(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_uppercase()) || name.contains(['-', '.'])
}

fn is_host(h: Host, tag: &SyntaxNode, text: &str) -> bool {
    let Some((name, _)) = tag_name(tag, text) else {
        return false;
    };
    match h {
        Host::Template => name == "template",
        Host::NotSlot => name != "slot",
        Host::Html => !is_component(name) && !matches!(name, "template" | "component"),
        Host::FileInput => {
            name.eq_ignore_ascii_case("input")
                && attrs(tag, text).iter().any(|a| {
                    a.full == "type" && a.value().is_some_and(|v| v.eq_ignore_ascii_case("file"))
                })
        }
    }
}

/// The start tag of the element before `tag`'s, when that is an element (blank space and
/// comments between them do not count).
fn previous_tag(tag: &SyntaxNode) -> Option<SyntaxNode> {
    let mut prev = tag.parent()?.prev_sibling();
    while let Some(n) = prev {
        match n.kind() {
            k::ELEMENT => return n.child(k::START_TAG),
            k::CHUNK
                if n.tokens()
                    .all(|t| matches!(t.kind(), k::WHITESPACE | k::NEWLINE | k::COMMENT)) =>
            {
                prev = n.prev_sibling();
            }
            _ => return None,
        }
    }
    None
}

impl Directive {
    fn violated(&self, a: &Attr<'_>, tag: &SyntaxNode, text: &str) -> bool {
        let (arg, mods) = a.parts();
        let mods: Vec<&str> = mods.collect();
        let value = a.value();
        let blank = value.is_some_and(|v| v.trim().is_empty());
        let empty = blank || value.is_none();
        let bad_value = match self.value {
            Value::Any => false,
            Value::Required => empty,
            Value::Forbidden => value.is_some(),
            Value::UnlessArg => blank || value.is_none() && arg.is_none(),
            Value::UnlessModifier => blank || value.is_none() && mods.is_empty(),
            Value::Iteration => !value.is_some_and(|v| {
                let words: Vec<&str> = v.split_whitespace().collect();
                words.len() >= 3
                    && words[1..words.len() - 1]
                        .iter()
                        .any(|w| *w == "in" || *w == "of")
            }),
            Value::VuePrefixed => value.is_some_and(|v| !v.starts_with("vue:")),
        };
        let bad_modifiers = match &self.modifiers {
            Modifiers::Any => false,
            Modifiers::None => !mods.is_empty(),
            Modifiers::Only(ok) => mods.iter().any(|m| !ok.iter().any(|o| o == m)),
        } || mods.iter().any(|m| self.banned.iter().any(|b| b == m))
            || self.numeric && mods.iter().any(|m| m.bytes().all(|b| b.is_ascii_digit()));
        self.unconditional
            || bad_value
            || bad_modifiers
            || self.no_arg && arg.is_some()
            || self.bad_host.is_some_and(|h| is_host(h, tag, text))
            || !self.with.is_empty() && has(&attrs(tag, text), &self.with)
            || !self.after.is_empty()
                && !previous_tag(tag).is_some_and(|t| has(&attrs(&t, text), &self.after))
    }
}

impl Rule for Template {
    fn meta(&self) -> &'static RuleMeta {
        self.meta
    }

    fn kinds(&self) -> &[SyntaxKind] {
        match self.check {
            Check::LoneTemplate | Check::TextareaMustache | Check::ComponentIs => &[k::START_TAG],
            _ => &[k::ATTRIBUTE],
        }
    }

    fn check(&self, node: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        if self.vue_only
            && ctx
                .file
                .path
                .extension()
                .is_none_or(|e| e.to_str() != Some("vue"))
        {
            return;
        }
        let text = ctx.file.text;
        match &self.check {
            Check::LoneTemplate => {
                let Some((name, range)) = tag_name(node, text) else {
                    return;
                };
                let nested = node
                    .parent()
                    .and_then(|e| e.parent())
                    .is_some_and(|p| p.kind() == k::ELEMENT);
                let directive = |a: &Attr<'_>| {
                    matches!(
                        a.name,
                        "v-if"
                            | "v-else"
                            | "v-else-if"
                            | "v-for"
                            | "v-slot"
                            | "v-pre"
                            | "v-is"
                            | "slot"
                            | "slot-scope"
                            | "scope"
                            | "is"
                    ) || a.name.starts_with("shadowroot")
                };
                if name == "template" && nested && !attrs(node, text).iter().any(directive) {
                    ctx.report(range, self.message.clone());
                }
            }
            Check::TextareaMustache => {
                if tag_name(node, text).is_some_and(|(n, _)| n.eq_ignore_ascii_case("textarea")) {
                    for c in node.parent().into_iter().flat_map(|e| e.children()) {
                        if c.kind() == k::INTERPOLATION {
                            ctx.report(c.text_range(), self.message.clone());
                        }
                    }
                }
            }
            Check::ComponentIs => {
                let Some((name, range)) = tag_name(node, text) else {
                    return;
                };
                let is = |a: &Attr<'_>| {
                    a.name == "v-is"
                        || a.name == "is"
                        || a.name == "v-bind" && a.parts().0 == Some("is")
                };
                if name == "component" && !attrs(node, text).iter().any(is) {
                    ctx.report(range, self.message.clone());
                }
            }
            check => {
                let s = code(node, text).0;
                let candidate = match check {
                    Check::Directive(d) => d.prefixes.iter().any(|p| s.starts_with(p.as_str())),
                    Check::Shorthand { from, .. } => s.starts_with(from.as_str()),
                    Check::KeyOnChild => s.starts_with(":key") || s.starts_with("v-bind:key"),
                    Check::ChildContent => s.starts_with("v-html") || s.starts_with("v-text"),
                    _ => s
                        .bytes()
                        .take_while(|b| !b.is_ascii_whitespace() && *b != b'=')
                        .any(|b| b.is_ascii_uppercase()),
                };
                if !candidate {
                    return;
                }
                let Some(a) = Attr::parse(node, text) else {
                    return;
                };
                let Some(tag) = node.parent() else {
                    return;
                };
                let hit = match check {
                    Check::Directive(d) => {
                        a.name == d.name
                            && d.host.is_none_or(|h| is_host(h, &tag, text))
                            && d.violated(&a, &tag, text)
                    }
                    Check::Shorthand { from, .. } => a.full.starts_with(from.as_str()),
                    Check::Hyphenation => {
                        let target = match (a.directive, a.name) {
                            (false, _) => Some(a.full),
                            (true, "v-bind") => a.parts().0,
                            _ => None,
                        };
                        target.is_some_and(|t| {
                            !t.starts_with('[')
                                && t.as_bytes()
                                    .windows(2)
                                    .any(|w| w[0].is_ascii_lowercase() && w[1].is_ascii_uppercase())
                        }) && tag_name(&tag, text).is_some_and(|(n, _)| is_component(n))
                    }
                    Check::KeyOnChild => {
                        a.name == "v-bind"
                            && a.parts().0 == Some("key")
                            && tag
                                .parent()
                                .and_then(|e| e.parent())
                                .and_then(|g| g.child(k::START_TAG))
                                .is_some_and(|t| {
                                    is_host(Host::Template, &t, text)
                                        && attrs(&t, text).iter().any(|x| x.name == "v-for")
                                })
                    }
                    Check::ChildContent => {
                        matches!(a.name, "v-html" | "v-text")
                            && tag.parent().is_some_and(|e| {
                                e.children().any(|c| match c.kind() {
                                    k::ELEMENT | k::INTERPOLATION => true,
                                    k::CHUNK => !text[c.text_range()].trim().is_empty(),
                                    _ => false,
                                })
                            })
                    }
                    _ => false,
                };
                if !hit {
                    return;
                }
                if let Check::Shorthand { from, to } = check {
                    let range = TextRange::at(a.start, TextSize::of(from.as_str()));
                    ctx.report_fix(
                        range,
                        self.message.clone(),
                        ("Use the shorthand", vec![Edit::replace(range, to.as_str())]),
                    );
                } else {
                    ctx.report(a.range(), self.message.clone());
                }
            }
        }
    }
}
