//! The option schema and presets of a language, from the `options` and `presets` keys of its
//! `lang.yaml`: `src/schema.generated.rs` holds one const per option name and `option_schema()`,
//! which the crate includes where its formatter lives (docs/ai/architecture/adding-a-language.md).

use crate::lang_crate::Lang;
use anyhow::{Result, bail};
use serde_json::Value;

/// `shared::` constructors with no argument.
const SHARED_NO_ARG: [&str; 2] = ["indent_style", "end_of_line"];
/// Names `shared::NAMES` already has a const for.
const SHARED_NAMES: [&str; 10] = [
    "max_blank_lines",
    "line_width",
    "indent_size",
    "indent_style",
    "final_newline",
    "end_of_line",
    "bracket_spacing",
    "array_bracket_spacing",
    "brace_newline",
    "brace_same_line",
];

fn lit(v: &Value) -> String {
    match v {
        Value::String(s) => format!("{s:?}"),
        Value::Array(items) => format!(
            "&[{}]",
            items.iter().map(lit).collect::<Vec<_>>().join(", ")
        ),
        other => other.to_string(),
    }
}

fn strs(v: &Value) -> String {
    let items = v.as_array().into_iter().flatten();
    format!("&[{}]", items.map(lit).collect::<Vec<_>>().join(", "))
}

/// One `OptionDef` expression and the option's name.
fn def(entry: &Value) -> Result<(String, String)> {
    if let Some(name) = entry.as_str() {
        if !SHARED_NO_ARG.contains(&name) {
            bail!("option `{name}` needs a value");
        }
        return Ok((name.into(), format!("shared::{name}()")));
    }
    let Some((name, body)) = entry.as_object().and_then(|m| m.iter().next()) else {
        bail!("an option is a name or a one-key map: {entry}");
    };
    let expr = match body {
        // A shared option with its default (and the description, for `max_blank_lines`).
        Value::Array(args) => format!(
            "shared::{name}({})",
            args.iter().map(lit).collect::<Vec<_>>().join(", ")
        ),
        Value::Object(m) if !m.contains_key("doc") => bail!("option `{name}` has no `doc`"),
        Value::Object(m) => {
            let doc = lit(&m["doc"]);
            let up = name.to_uppercase();
            let default = m.get("default").map(lit).unwrap_or_default();
            if let Some(values) = m.get("values") {
                format!(
                    "OptionDef::choice(core_config::opt::{up}, {}, {default}, {doc})",
                    strs(values)
                )
            } else if let Some(r) = m.get("range").and_then(Value::as_array) {
                format!(
                    "OptionDef::int(core_config::opt::{up}, {default}, ({}, {}), {doc})",
                    lit(&r[0]),
                    lit(&r[1])
                )
            } else if m.get("kind").and_then(Value::as_str) == Some("words") {
                let words = m
                    .get("default")
                    .and_then(Value::as_array)
                    .map(|a| a.iter().map(lit).collect::<Vec<_>>().join(", "))
                    .unwrap_or_default();
                format!("OptionDef::words(core_config::opt::{up}, &[{words}], {doc})")
            } else if m.get("kind").and_then(Value::as_str) == Some("word-sets") {
                format!("OptionDef::word_sets(core_config::opt::{up}, {doc})")
            } else {
                format!("OptionDef::boolean(core_config::opt::{up}, {default}, {doc})")
            }
        }
        scalar => format!("shared::{name}({})", lit(scalar)),
    };
    let expr = match expr.starts_with("shared::") {
        true => expr,
        false => format!("core_config::{expr}"),
    };
    Ok((
        name.clone(),
        expr.replace("shared::", "core_config::shared::"),
    ))
}

fn pascal(name: &str) -> String {
    let word = |w: &str| w[..1].to_uppercase() + &w[1..];
    name.split('_')
        .filter(|w| !w.is_empty())
        .map(word)
        .collect()
}

/// The typed getter of an option declared in full: an enum for a choice, `bool` or `usize`.
/// A getter never panics: an option the resolved set lacks (code embedded in another language
/// reads the host's options) has its default.
fn getter(name: &str, body: &Value) -> String {
    let Some(m) = body.as_object() else {
        return String::new();
    };
    let konst = name.to_uppercase();
    let o = "o: &core_config::ResolvedOptions";
    let allow = "#[allow(dead_code)]\n";
    if let Some(values) = m.get("values").and_then(Value::as_array) {
        let words: Vec<&str> = values.iter().filter_map(Value::as_str).collect();
        let plain = |w: &&str| {
            w.starts_with(|c: char| c.is_ascii_lowercase())
                && w.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        };
        if words.is_empty() || !words.iter().all(plain) {
            return String::new();
        }
        let ty = pascal(name);
        let default = m.get("default").and_then(Value::as_str).unwrap_or(words[0]);
        let variants = words
            .iter()
            .map(|w| pascal(w))
            .collect::<Vec<_>>()
            .join(", ");
        let arms: String = words
            .iter()
            .filter(|w| **w != default)
            .map(|w| format!("        {w:?} => {ty}::{},\n", pascal(w)))
            .collect();
        let derive = "#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n";
        return format!(
            "{allow}{derive}pub enum {ty} {{ {variants} }}\n{allow}pub fn {name}({o}) -> {ty} {{\n    match o.text({konst}) {{\n{arms}        _ => {ty}::{},\n    }}\n}}\n",
            pascal(default)
        );
    }
    let default = m.get("default").map(lit).unwrap_or_default();
    if m.contains_key("range") {
        let default = if default.is_empty() {
            "0".into()
        } else {
            default
        };
        return format!(
            "{allow}pub fn {name}({o}) -> usize {{\n    o.uint({konst}, {default})\n}}\n"
        );
    }
    if m.contains_key("kind") {
        return String::new();
    }
    let default = if default.is_empty() {
        "false".into()
    } else {
        default
    };
    format!("{allow}pub fn {name}({o}) -> bool {{\n    o.flag({konst}, {default})\n}}\n")
}

/// The `kinds` of an option (`role: [Kind, ..]`, or `role: {word: [Kind, ..]}`): the constructs
/// the option governs, as `(option, role, word, kinds)` tuples for the shared shapes.
fn option_kinds(name: &str, body: &Value) -> String {
    let roles = body.get("kinds").and_then(Value::as_object);
    let mut out = String::new();
    for (role, set) in roles.into_iter().flatten() {
        let words: Vec<(&str, &Value)> = match set {
            Value::Object(m) => m.iter().map(|(w, v)| (w.as_str(), v)).collect(),
            other => vec![("", other)],
        };
        for (word, kinds) in words {
            out += &format!("    ({name:?}, {role:?}, {word:?}, {}),\n", strs(kinds));
        }
    }
    out
}

/// `src/schema.generated.rs` of a language with an `options` key.
pub fn generate(name: &str, l: &Lang) -> Result<Option<String>> {
    let Some(options) = l.yaml.get("options").and_then(Value::as_array) else {
        return Ok(None);
    };
    let mut consts = String::new();
    let mut defs = String::new();
    let mut getters = String::new();
    let mut rows = String::new();
    for entry in options {
        let (opt, expr) = def(entry)?;
        if !SHARED_NAMES.contains(&opt.as_str()) {
            consts += &format!(
                "#[allow(dead_code)]\npub const {}: core_config::OptId = core_config::opt::{};\n",
                opt.to_uppercase(),
                opt.to_uppercase()
            );
            if let Some(body) = entry.as_object().and_then(|m| m.values().next()) {
                getters += &getter(&opt, body);
                rows += &option_kinds(&opt, body);
            }
        }
        defs += &format!("        {expr},\n");
    }
    if !rows.is_empty() {
        consts +=
            &format!("pub const OPTION_KINDS: core_format::rules::OptionKinds = &[\n{rows}];\n");
    }
    let presets = l.yaml.get("presets").and_then(Value::as_array);
    let presets = presets.into_iter().flatten().filter_map(Value::as_str);
    let presets: String = presets
        .map(|p| format!("        core_config::Preset::new(\"{name}/{p}\", include_str!(\"../presets/{p}.yaml\")),\n"))
        .collect();
    Ok(Some(format!(
        "// Generated from lang.yaml by `cargo xtask codegen`.\n{consts}{getters}\n#[rustfmt::skip]\npub fn option_schema() -> core_config::OptionSchema {{\n    core_config::OptionSchema::lazy(|| {{\n        core_config::OptionSchema::new(vec![\n{defs}        ])\n        .with_presets(vec![\n{presets}        ])\n    }})\n}}\n"
    )))
}

/// The option ids: `crates/core-config/src/opt_ids.generated.rs`, one id per distinct option name
/// of the languages' `options` and of `crates/core-config/options.txt` (the names shared code
/// reads), sorted by name.
pub fn ids(root: &std::path::Path, langs: &[Lang]) -> Result<(std::path::PathBuf, String)> {
    let shared = std::fs::read_to_string(root.join("crates/core-config/options.txt"))?;
    let mut names: std::collections::BTreeSet<String> = shared
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(String::from)
        .collect();
    for l in langs {
        for entry in l
            .yaml
            .get("options")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            names.insert(def(entry)?.0);
        }
    }
    let mut consts = String::new();
    for (i, n) in names.iter().enumerate() {
        consts += &format!("pub const {}: OptId = OptId({i});\n", n.to_uppercase());
    }
    let list: String = names.iter().map(|n| format!("    {n:?},\n")).collect();
    Ok((
        "crates/core-config/src/opt_ids.generated.rs".into(),
        format!(
            "// Generated by `cargo xtask codegen` from the options of every lang.yaml and crates/core-config/options.txt.\n{consts}\n/// Option names sorted; an id is the index.\npub static NAMES: [&str; {}] = [\n{list}];\n",
            names.len()
        ),
    ))
}
