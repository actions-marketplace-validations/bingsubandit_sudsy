//! Vue : HTML plus template syntax. The provider is an HTML one with another
//! tree-sitter grammar mapped onto the same tree (`grammars/html.grammar`), its own file
//! matchers and `vue` / `svelte` config sections on top of `html`.

use crate::MAPPING;
use crate::format::{ANGULAR_FORMATTER, SVELTE_FORMATTER, VUE_FORMATTER, option_schema};
use core_config::{OptionDef, OptionSchema};
use core_treesitter::{Mapping, Spec, provider_of};
use provider_api::{ParserProvider, ParserSpec};

/// HTML's node table with the nodes of another grammar added (and the tokens it wraps in nodes).
fn mapping(
    extra: &[(&'static str, &'static str)],
    wrap: &[(&'static str, &'static str)],
) -> &'static Mapping {
    let mut kinds = MAPPING.kinds.to_vec();
    kinds.extend_from_slice(extra);
    Box::leak(Box::new(Mapping {
        whitespace: MAPPING.whitespace,
        newline: MAPPING.newline,
        eof: MAPPING.eof,
        unknown: MAPPING.unknown,
        kinds: Box::leak(kinds.into_boxed_slice()),
        wrap: Box::leak([MAPPING.wrap, wrap].concat().into_boxed_slice()),
    }))
}

/// The options of HTML and, in front of the shared ones, `extra`.
fn schema(extra: Vec<OptionDef>) -> OptionSchema {
    let base = option_schema();
    let mut defs = base.defs().to_vec();
    defs.extend(extra);
    OptionSchema::new(defs)
}

macro_rules! spec {
    ($name:literal, $krate:ident, $mapping:expr, $formatter:expr) => {
        Spec {
            crate_name: env!("CARGO_PKG_NAME"),
            language: || core_packs::language!($name, $krate::LANGUAGE),
            grammar: core_syntax::grammars::HTML,
            mapping: $mapping,
            extensions: &[$name],
            file_names: &[],
            schema: option_schema(),
            formatter: $formatter,
            hooks: &crate::hooks::HtmlHooks,
            rules: crate::rules::RULES,
            pattern_files: provider_api::pattern_files!(),
            families: crate::rules::FAMILIES,
            validate: None,
        }
    };
}

/// `.vue` single-file components.
pub fn vue_provider() -> ParserProvider {
    let m = mapping(
        &[
            ("template_element", "Element"),
            ("directive_attribute", "Attribute"),
            ("directive_name", "AttrName"),
            ("directive_value", "AttrName"),
            ("dynamic_directive_value", "AttrName"),
            ("directive_modifier", "AttrName"),
            ("\":\"", "AttrName"),
            ("\".\"", "AttrName"),
            ("\"@\"", "AttrName"),
            ("\"#\"", "AttrName"),
            ("interpolation", "Interpolation"),
        ],
        &[],
    );
    let options = vec![OptionDef::boolean(
        crate::format::INDENT_SCRIPT_AND_STYLE,
        false,
        "Indent the content of the root `<script>` and `<style>` blocks of a `.vue` file by one level.",
    )];
    let mut spec = spec!("vue", tree_sitter_vue, m, &VUE_FORMATTER);
    spec.schema = schema(options);
    provider_of(spec).with_dialect("vue", "html")
}

/// `.svelte` components.
pub fn svelte_provider() -> ParserProvider {
    let mut kinds = vec![
        ("expression", "MustacheText"),
        ("html_tag", "MustacheText"),
        ("const_tag", "MustacheText"),
        ("debug_tag", "MustacheText"),
        ("render_tag", "MustacheText"),
    ];
    kinds.extend(
        [
            "if_start",
            "else_if_start",
            "else_start",
            "if_end",
            "each_start",
            "each_end",
            "await_start",
            "then_start",
            "catch_start",
            "await_end",
            "key_start",
            "key_end",
            "snippet_start",
            "snippet_end",
        ]
        .map(|n| (n, "BlockTagText")),
    );
    kinds.extend(
        [
            "if_statement",
            "each_statement",
            "await_statement",
            "key_statement",
            "snippet_statement",
        ]
        .map(|n| (n, "Block")),
    );
    let m = mapping(
        &kinds,
        &[("MustacheText", "Mustache"), ("BlockTagText", "BlockTag")],
    );
    let options = vec![OptionDef::boolean(
        crate::format::INDENT_SCRIPT_AND_STYLE,
        true,
        "Indent the content of the root `<script>` and `<style>` blocks of a `.svelte` file by one level.",
    )];
    let mut spec = spec!("svelte", tree_sitter_svelte, m, &SVELTE_FORMATTER);
    spec.schema = schema(options);
    provider_of(spec).with_dialect("svelte", "html")
}

/// Angular component templates (`*.component.html`): oxc-htmlish-parser's Angular reading as our
/// HTML tree (`crate::angular`).
pub fn angular_provider() -> ParserProvider {
    let spec = ParserSpec {
        crate_name: env!("CARGO_PKG_NAME"),
        grammar: core_syntax::grammars::HTML,
        extensions: &[],
        file_names: &["*.component.html"],
        schema: option_schema(),
        formatter: &ANGULAR_FORMATTER,
        hooks: &crate::hooks::HtmlHooks,
        rules: crate::rules::RULES,
        pattern_files: provider_api::pattern_files!(),
        families: crate::rules::FAMILIES,
    };
    ParserProvider::new(spec, crate::angular::parse_file).with_dialect("angular", "html")
}
