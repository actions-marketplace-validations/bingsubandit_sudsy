//! Rules on plain values: `no-empty-values`, `float-style` (opt-in) and `no-octal-values`.
//!
//! concept: new no-empty-values, new float-style
//! what: no-empty-values: A mapping key or sequence entry without a value (an implicit null)
//! what: float-style: Restricts how plain floats are written

use crate::generated::{BlockMappingEntry, BlockSequenceEntry, FlowPair, Scalar, kind};
use crate::rules::has_tag;
use core_rules::Re;
use core_rules::rule;
use core_rules::{Check, Edit, Opt, OptionDecl, RuleContext};
use core_syntax::ast::AstNode;
use core_syntax::{SyntaxNode, SyntaxNodeExt, SyntaxToken, TextSize};
use std::sync::LazyLock;

const fn flag(name: &'static str, default: bool, description: &'static str) -> OptionDecl {
    OptionDecl {
        name,
        default: Opt::Bool(default),
        check: Check::Any,
        description,
    }
}

rule! {
    pub NoEmptyValues {
        id: "no-empty-values",
        category: Style,
        default: Off,
        description: "Reports a mapping key or sequence entry without a value (an implicit null), at the \
                  `:` or `-`. A value with an anchor or tag is not empty. The options \
                  `block_mappings`, `flow_mappings` and `block_sequences` choose what is checked.",
        why: "An empty value is `null`, but it is easy to read as an omission or a value that was \
                  lost while editing, and parsers of config files may treat `null` and a missing key \
                  differently.\n\
                  To fix, write `null` or `~`, or give the key its value.",
        example: "key:      # reported; write `key: null`\nlist:\n  -       # reported",
        options: [
            flag("block_mappings", true, "Check block mappings."),
            flag("flow_mappings", true, "Check flow mappings (`{a: }`)."),
            flag("block_sequences", true, "Check block sequences."),
    ],
        kinds: &[
            kind::BLOCK_MAPPING_ENTRY,
            kind::FLOW_PAIR,
            kind::BLOCK_SEQUENCE_ENTRY,
        ],
    }

    fn check(&self, node: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        let (marker, empty, option, what) = match node.kind() {
            kind::BLOCK_MAPPING_ENTRY => (
                kind::COLON,
                BlockMappingEntry::cast(node.clone()).is_some_and(|e| e.value().is_none()),
                "block_mappings",
                "block mapping",
            ),
            kind::FLOW_PAIR => (
                kind::COLON,
                FlowPair::cast(node.clone()).is_some_and(|e| e.value().is_none()),
                "flow_mappings",
                "flow mapping",
            ),
            _ => (
                kind::DASH,
                BlockSequenceEntry::cast(node.clone()).is_some_and(|e| e.value().is_none()),
                "block_sequences",
                "block sequence",
            ),
        };
        if !empty || !ctx.options.bool(option) {
            return;
        }
        if let Some(token) = node
            .token(marker)
        {
            ctx.report(token.text_range(), format!("empty value in {what}"));
        }
    }
}

/// The plain, untagged scalar of `node` (not a quoted one, not a key's alias).
fn plain_untagged(node: &SyntaxNode) -> Option<SyntaxToken> {
    let token = Scalar::cast(node.clone())?.value()?;
    (token.kind() == kind::PLAIN && !has_tag(node)).then_some(token)
}

fn pattern(source: &str) -> Re {
    Re::lazy(source).expect("the patterns are valid")
}

static NUMERAL_BEFORE_DECIMAL: LazyLock<Re> =
    LazyLock::new(|| pattern(r"^[-+]?\.[0-9]+([eE][-+]?[0-9]+)?$"));
static SCIENTIFIC: LazyLock<Re> =
    LazyLock::new(|| pattern(r"^[-+]?(\.[0-9]+|[0-9]+(\.[0-9]*)?)[eE][-+]?[0-9]+$"));
static INF: LazyLock<Re> = LazyLock::new(|| pattern(r"^[-+]?\.(inf|Inf|INF)$"));
static NAN: LazyLock<Re> = LazyLock::new(|| pattern(r"^\.(nan|NaN|NAN)$"));

rule! {
    pub FloatStyle {
        id: "float-style",
        category: Style,
        default: Off,
        fix: Safe,
        description: "Restricts how plain floats are written. Every option is off until set: \
                  `require_numeral_before_decimal` reports `.5`, `forbid_scientific_notation` reports \
                  `1e3`, `forbid_nan` reports `.nan` and `forbid_inf` reports `.inf`. Quoted and \
                  tagged scalars are not checked. The fix for `.5` is `0.5`.",
        why: "YAML 1.1 and 1.2 parsers read some float spellings differently (`1e3` is a string in \
                  YAML 1.1, since it requires a dot), and `.5` is hard to see. A project can choose \
                  the spellings that every parser it uses reads the same way.",
        example: "x: .5   # with require_numeral_before_decimal; becomes 0.5",
        options: [
            flag(
                "require_numeral_before_decimal",
                false,
                "Report `.5`, `-.5e3`: a digit must come before the decimal point.",
            ),
            flag(
                "forbid_scientific_notation",
                false,
                "Report `1e3`, `1.5E-2`, `.5e3`.",
            ),
            flag("forbid_nan", false, "Report `.nan`, `.NaN`, `.NAN`."),
            flag("forbid_inf", false, "Report `.inf`, `-.Inf`, `+.INF`."),
    ],
        kinds: &[kind::SCALAR],
    }

    fn check(&self, node: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        let Some(token) = plain_untagged(node) else {
            return;
        };
        let (text, range) = (token.text(), token.text_range());
        let o = ctx.options;
        if o.bool("forbid_nan") && NAN.is_match(text) {
            ctx.report(range, format!("forbidden not a number value `{text}`"));
        }
        if o.bool("forbid_inf") && INF.is_match(text) {
            ctx.report(range, format!("forbidden infinite value `{text}`"));
        }
        if o.bool("forbid_scientific_notation") && SCIENTIFIC.is_match(text) {
            ctx.report(range, format!("forbidden scientific notation `{text}`"));
        }
        if o.bool("require_numeral_before_decimal") && NUMERAL_BEFORE_DECIMAL.is_match(text) {
            let at = range.start() + TextSize::new(u32::from(text.starts_with(['-', '+'])));
            ctx.report_fix(
                range,
                format!("decimal `{text}` is missing the 0 before the point"),
                ("Add the 0", vec![Edit::insert(at, "0")]),
            );
        }
    }
}
