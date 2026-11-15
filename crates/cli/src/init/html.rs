//! html-validate (`.htmlvalidate.json`), HTMLHint (`.htmlhintrc`) and @html-eslint (a legacy JSON or
//! YAML ESLint config): their rules become `html.lint.rules`, which Vue and Svelte files inherit.
//! JavaScript configs are not read.

use super::eslint;
use super::out::Out;
use anyhow::{Result, bail};
use serde_json::{Map, Value};

pub const HTML_VALIDATE_FILES: [&str; 1] = [".htmlvalidate.json"];
pub const HTML_VALIDATE_JS: [&str; 3] =
    [".htmlvalidate.js", ".htmlvalidate.cjs", ".htmlvalidate.mjs"];
pub const HTMLHINT_FILES: [&str; 1] = [".htmlhintrc"];

/// html-validate's named pattern styles, which Sudsy writes as a regex.
const NAMED_STYLES: [&str; 4] = ["kebabcase", "camelcase", "underscore", "bem"];

fn object(config: &Value) -> Result<&Map<String, Value>> {
    match config.as_object() {
        Some(o) => Ok(o),
        None => bail!("the config is not an object"),
    }
}

pub fn html_validate(config: &Value, out: &mut Out) -> Result<()> {
    let root = object(config)?;
    let extends = match root.get("extends") {
        Some(Value::String(s)) => vec![s.as_str()],
        Some(Value::Array(a)) => a.iter().filter_map(Value::as_str).collect(),
        _ => vec![],
    };
    for name in extends {
        match name {
            "html-validate:recommended" => out.extend("html/html-validate-recommended"),
            "html-validate:standard" => out.extend("html/html-validate-standard"),
            _ => out.skip_why(
                name,
                "a shared config or preset is not followed; its rules are not migrated",
            ),
        }
    }
    for (name, value) in root
        .get("rules")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
    {
        if let Some((level, opts)) = eslint::setting(value) {
            rule(name, name, level, opts, out);
        }
    }
    for key in ["elements", "plugins", "transform"] {
        if root.contains_key(key) {
            out.skip_why(key, "not migrated; set the rules by hand");
        }
    }
    Ok(())
}

/// HTMLHint: `"rule": true | false | <option>`; only the listed rules run, so an unlisted rule
/// that Sudsy turns on by default is not switched off.
pub fn htmlhint(config: &Value, out: &mut Out) -> Result<()> {
    for (name, value) in object(config)? {
        let (level, opts): (&str, &[Value]) = match value {
            Value::Bool(false) => ("off", &[]),
            Value::Bool(true) => ("error", &[]),
            v => ("error", std::slice::from_ref(v)),
        };
        rule(name, name, level, opts, out);
    }
    out.note(
        "HTMLHint runs only the rules the config lists; Sudsy's own defaults apply to the others",
    );
    Ok(())
}

/// @html-eslint: the `@html-eslint/` rules of the config and of the overrides for HTML files.
pub fn html_eslint(config: &Value, out: &mut Out) -> Result<()> {
    let root = object(config)?;
    let mut sets = vec![root];
    for entry in root
        .get("overrides")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let files = entry.get("files").map(Value::to_string).unwrap_or_default();
        if let (true, Some(o)) = (files.contains(".html"), entry.as_object()) {
            sets.push(o);
        }
    }
    for set in sets {
        for (name, value) in set
            .get("rules")
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
        {
            let Some(id) = name.strip_prefix("@html-eslint/") else {
                continue;
            };
            if let Some((level, opts)) = eslint::setting(value) {
                rule(name, id, level, opts, out);
            }
        }
    }
    if root.get("extends").is_some() {
        out.note("`extends` is not followed; the rules of a shared config are not migrated");
    }
    Ok(())
}

/// One rule setting; `name` is the config key, `id` the catalog entry.
fn rule(name: &str, id: &str, level: &str, opts: &[Value], out: &mut Out) {
    let Some(entry) = out.entry(id, "not a rule Sudsy knows") else {
        return;
    };
    let Some(sudsy) = entry.sudsy.strip_prefix("rule: ") else {
        if level != "off" {
            out.skip(id);
        }
        return;
    };
    if sudsy == "syntax-error" {
        if level == "off" {
            out.skip_why(name, "syntax errors are always reported");
        }
        return;
    }
    out.rule(id, "html", sudsy, level);
    if level == "off" || opts.is_empty() {
        return;
    }
    let pattern = opts[0].get("pattern").and_then(Value::as_str);
    match (sudsy.ends_with("-pattern"), pattern) {
        (true, Some(p)) if !NAMED_STYLES.contains(&p) => {
            out.rule_opt(id, "html", sudsy, "pattern", p.into());
        }
        _ => out.skip_why(
            &format!("{name} options"),
            "Sudsy's rule has no option for them; set them by hand",
        ),
    }
}
