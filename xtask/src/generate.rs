//! `cargo xtask generate <language> <dir>`: a checked-in data table of a language built from the npm
//! packages installed in `<dir>` (`npm i <packages>`), as its `lang.yaml` key `generate:` declares:
//! `output` (a path below the crate), `header` (the first line; `{<package>}` is the package name and
//! its version), `sections` (a table section per entry: `name` and `from`, sources that are a JSON `file`
//! below `node_modules/` with optional `at` (a key), `prefix` / `not_prefix` (a name filter) and
//! `except` (`file` and `in_each`: the names under that key of every entry of another file are
//! left out)), and how names are normalized (`strip_leading` characters, `cut_at` a character,
//! `drop_prefix`; names are lowercased). Pinned and refreshed deliberately: the table is checked in, so
//! builds and `codegen --check` need no network.

use crate::lang_crate::{self, text};
use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

fn json(dir: &Path, file: &str) -> Result<Value> {
    let path = dir.join("node_modules").join(file);
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    Ok(serde_json::from_str(&text)?)
}

/// The keys of an object, or the strings of an array.
fn names(v: &Value) -> Vec<&str> {
    match v {
        Value::Object(o) => o.keys().map(String::as_str).collect(),
        Value::Array(a) => a.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    }
}

/// How `generate:` turns a source name into an entry.
struct Normalize<'a> {
    strip_leading: Vec<char>,
    cut_at: Option<char>,
    drop_prefix: &'a str,
}

impl Normalize<'_> {
    fn entry(&self, name: &str) -> Option<String> {
        let lower = name.to_ascii_lowercase();
        let base = lower.trim_start_matches(self.strip_leading.as_slice());
        let base = base.split(self.cut_at.unwrap_or('\0')).next().unwrap_or("");
        let dropped = !self.drop_prefix.is_empty() && base.starts_with(self.drop_prefix);
        (!base.is_empty() && !dropped).then(|| base.to_string())
    }
}

fn source_names(dir: &Path, source: &Value) -> Result<Vec<String>> {
    let all = json(dir, text(source, "file", ""))?;
    let node = source
        .get("at")
        .and_then(Value::as_str)
        .map_or(&all, |k| &all[k]);
    let mut left_out: Vec<String> = Vec::new();
    if let Some(except) = source.get("except") {
        let other = json(dir, text(except, "file", ""))?;
        let key = text(except, "in_each", "");
        for entry in other.as_object().into_iter().flat_map(|o| o.values()) {
            left_out.extend(names(&entry[key]).into_iter().map(String::from));
        }
    }
    let (prefix, not_prefix) = (text(source, "prefix", ""), text(source, "not_prefix", "\0"));
    Ok(names(node)
        .into_iter()
        .filter(|n| {
            n.starts_with(prefix) && !n.starts_with(not_prefix) && !left_out.iter().any(|l| l == n)
        })
        .map(String::from)
        .collect())
}

pub fn generate(root: &Path, language: &str, dir: &Path) -> Result<()> {
    let langs = lang_crate::load(root)?;
    let krate = format!("lang-{language}");
    let lang = langs
        .iter()
        .find(|l| l.krate == krate)
        .with_context(|| format!("no language crate {krate}"))?;
    let Some(spec) = lang.yaml.get("generate") else {
        bail!("{krate}/lang.yaml has no `generate:` key");
    };
    let normalize = Normalize {
        strip_leading: text(spec, "strip_leading", "").chars().collect(),
        cut_at: text(spec, "cut_at", "").chars().next(),
        drop_prefix: text(spec, "drop_prefix", ""),
    };
    let mut header = text(spec, "header", "").to_string();
    while let Some(start) = header.find('{') {
        let end = start
            + header[start..]
                .find('}')
                .context("unclosed `{` in header")?;
        let package = header[start + 1..end].to_string();
        let manifest = json(dir, &format!("{package}/package.json"))?;
        let version = format!("{package} {}", manifest["version"].as_str().unwrap_or("?"));
        header.replace_range(start..=end, &version);
    }
    let mut out = format!("# {header}\n");
    let sections = spec.get("sections").and_then(Value::as_array);
    for section in sections.into_iter().flatten() {
        let mut set = BTreeSet::new();
        for source in section
            .get("from")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let found = source_names(dir, source)?;
            set.extend(found.iter().filter_map(|n| normalize.entry(n)));
        }
        out.push_str(&format!("[{}]\n", text(section, "name", "")));
        let mut line = String::new();
        for name in set {
            if line.len() + name.len() > 100 {
                out.push_str(line.trim_end());
                out.push('\n');
                line.clear();
            }
            line.push_str(&name);
            line.push(' ');
        }
        out.push_str(line.trim_end());
        out.push('\n');
    }
    let path = root
        .join("crates")
        .join(&krate)
        .join(text(spec, "output", ""));
    std::fs::write(&path, out).with_context(|| format!("writing {}", path.display()))
}
