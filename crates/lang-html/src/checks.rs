//! The `meta` family: checks of an element's name and attributes against the element metadata
//! (`meta.rs`). Param `check`:
//! - `unknown-element`: a name that is no HTML element (what is inside `<svg>` and `<math>` is left out);
//! - `element-name`: a name that is neither an HTML element nor a valid custom element name
//!   (`a-b`: lowercase letters, digits, `-`, `.` and `_`, with a `-`);
//! - `deprecated-element`; `deprecated-attr`;
//! - `required-attr`: an attribute the element must have;
//! - `boolean-style`: a boolean attribute written with a value (`checked="checked"`);
//! - `empty-style`: an attribute that may be empty (`hidden`, `crossorigin`) written as `=""`;
//! - `allowed-values`: a value that is not one of those the attribute is limited to;
//! - `unsupported-feature`: an element, attribute or value that browsers do not all support at the `available` level (`baseline.txt`, `core_rules::feature_support`).
//!
//! The message has `{name}` (the element), `{attr}` and `{value}`; `unsupported-feature` has `{feature}` and `{available}`.
//!
//! concept: new element-metadata-checks
//! what: Element names and attributes checked against element metadata.
//! tools: html-validate element-name, htmlhint

use crate::dom;
use crate::generated::kind as k;
use crate::meta;
use crate::template;
use core_rules::family::comma_list;
use core_rules::feature_support::{AttrRef, Availability, SupportTable};
use core_rules::{Family, Params, Rule, RuleContext, RuleMeta};
use core_syntax::{SyntaxKind, SyntaxNode, SyntaxNodeExt, TextRange};

pub const FAMILY: Family = Family {
    name: "meta",
    build,
};

#[derive(Clone, Copy)]
enum Check {
    UnknownElement,
    ElementName,
    DeprecatedElement,
    DeprecatedAttr,
    RequiredAttr,
    BooleanStyle,
    EmptyStyle,
    AllowedValues,
    UnsupportedFeature,
}

struct Meta {
    meta: &'static RuleMeta,
    check: Check,
    message: String,
    /// Attributes (and, for the element checks, every element name) a framework owns, not judged in files with a suffix of `owned_in`.
    owned: Vec<String>,
    owned_in: Vec<String>,
}

fn build(meta: &'static RuleMeta, mut p: Params) -> Result<Box<dyn Rule>, String> {
    use Check::*;
    let check = p.choice(
        "check",
        true,
        &[
            ("unknown-element", UnknownElement),
            ("element-name", ElementName),
            ("deprecated-element", DeprecatedElement),
            ("deprecated-attr", DeprecatedAttr),
            ("required-attr", RequiredAttr),
            ("boolean-style", BooleanStyle),
            ("empty-style", EmptyStyle),
            ("allowed-values", AllowedValues),
            ("unsupported-feature", UnsupportedFeature),
        ],
    )?;
    let rule = Meta {
        meta,
        check,
        message: p.text("message")?,
        owned: p
            .optional("owned")
            .map_or_else(Vec::new, |l| comma_list(&l)),
        owned_in: p
            .optional("owned-in")
            .map_or_else(Vec::new, |l| comma_list(&l)),
    };
    p.done()?;
    Ok(Box::new(rule))
}

impl Rule for Meta {
    fn meta(&self) -> &'static RuleMeta {
        self.meta
    }

    fn kinds(&self) -> &[SyntaxKind] {
        &[k::START_TAG]
    }

    fn check(&self, tag: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        let text = ctx.file.text;
        let Some((name, name_range)) = dom::tag_name(tag, text) else {
            return;
        };
        let say = |attr: &str, value: &str| {
            self.message
                .replace("{name}", &name)
                .replace("{attr}", attr)
                .replace("{value}", value)
        };
        match self.check {
            Check::UnknownElement | Check::ElementName if self.owns(ctx) => {}
            Check::UnknownElement => {
                if !name.contains(':') && !meta::has("elements", &name) && !foreign(tag, text) {
                    ctx.report(name_range, say("", ""));
                }
            }
            Check::ElementName => {
                if !meta::has("elements", &name) && !custom_name(&name) && !foreign(tag, text) {
                    ctx.report(name_range, say("", ""));
                }
            }
            Check::DeprecatedElement => {
                if meta::has("deprecated", &name) {
                    ctx.report(name_range, say("", ""));
                }
            }
            Check::RequiredAttr => {
                let attrs = dom::attrs(tag, text);
                for required in meta::required(&name) {
                    if !attrs.iter().any(|a| a.name == required) {
                        ctx.report(name_range, say(required, ""));
                    }
                }
            }
            Check::UnsupportedFeature => self.unsupported(&name, name_range, tag, ctx),
            _ => {
                for a in dom::attrs(tag, text) {
                    let value = a.value.unwrap_or("");
                    let wrong = match self.check {
                        Check::DeprecatedAttr => meta::flagged("deprecated-attr", &name, &a.name),
                        Check::BooleanStyle => {
                            a.value.is_some() && meta::flagged("boolean", &name, &a.name)
                        }
                        Check::EmptyStyle => {
                            value.is_empty()
                                && a.value.is_some()
                                && meta::flagged("omit", &name, &a.name)
                        }
                        _ => meta::allowed(&name, &a.name).is_some_and(|allowed| {
                            core_rules::allowed_values::wrong(
                                allowed,
                                a.value,
                                (&text[..a.key.start().into()], &text[a.key.end().into()..]),
                                || meta::flagged("omit", &name, &a.name),
                                || meta::flagged("list", &name, &a.name),
                            )
                        }),
                    };
                    if wrong {
                        ctx.report(a.key, say(&a.name, value));
                    }
                }
            }
        }
    }
}

impl Meta {
    fn owns(&self, ctx: &RuleContext<'_>) -> bool {
        (self.owned_in.iter()).any(|f| template::path_has_suffix(ctx.file.path, f))
    }
    /// The unsupported features of an element and its attributes (`core_rules::feature_support`).
    fn unsupported(
        &self,
        name: &str,
        name_range: TextRange,
        tag: &SyntaxNode,
        ctx: &mut RuleContext<'_>,
    ) {
        static TABLE: SupportTable = SupportTable::new(include_str!("../data/baseline.txt"));
        let available = ctx.options.string("available").to_string();
        let attrs = dom::attrs(tag, ctx.file.text);
        let found = TABLE.features(
            Availability::parse(&available),
            name,
            name_range,
            attrs.iter().map(|a| AttrRef {
                name: &a.name,
                name_range: a.key,
                value: a.value,
                value_range: a.value_range,
            }),
            if self.owns(ctx) { &self.owned } else { &[] },
        );
        for (range, feature) in found {
            let message = self
                .message
                .replace("{feature}", &feature)
                .replace("{available}", &available);
            ctx.report(range, message);
        }
    }
}

/// Is the element inside `<svg>` or `<math>`, whose elements the metadata does not list?
fn foreign(tag: &SyntaxNode, text: &str) -> bool {
    tag.ancestors()
        .skip(2)
        .filter(|a| a.kind() == k::ELEMENT)
        .any(|e| {
            e.child(k::START_TAG)
                .and_then(|t| dom::tag_name(&t, text))
                .is_some_and(|(n, _)| n == "svg" || n == "math")
        })
}

fn custom_name(name: &str) -> bool {
    name.contains('-')
        && name.starts_with(|c: char| c.is_ascii_lowercase())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "-._".contains(c))
}
