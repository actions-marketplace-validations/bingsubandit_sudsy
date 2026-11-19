//! What the languages state as data (grammar lines and tags, `format.rules` sections and keys,
//! `lang.yaml` options, rule files) and the parity catalogs, read as text.

use super::{Lang, Module};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Default)]
struct LangData {
    lines: BTreeSet<String>,
    glines: Vec<String>,
    pattern_rules: bool,
    tags: BTreeSet<String>,
    sections: BTreeSet<String>,
    keys: BTreeSet<String>,
    options: BTreeSet<String>,
    families: BTreeSet<String>,
    files: BTreeSet<String>,
    uses: BTreeSet<String>,
    rust: String,
}

pub struct Langs {
    langs: BTreeMap<Lang, LangData>,
    rule_langs: BTreeMap<String, BTreeSet<String>>,
}

fn read(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_default()
}

fn rs_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            rs_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// `a::{b, c::{d, e}}` as the list of its paths.
fn expand(s: &str) -> Vec<String> {
    let s = s.trim();
    let Some(open) = s.find('{') else {
        let s = s.split(" as ").next().unwrap_or(s).trim();
        return vec![s.trim_end_matches("::self").to_string()];
    };
    let close = s.rfind('}').unwrap_or(s.len());
    if close <= open {
        return vec![s.to_string()];
    }
    let prefix = &s[..open];
    let mut parts = Vec::new();
    let (mut depth, mut start) = (0, open + 1);
    for (i, c) in s[open + 1..close].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(&s[start..open + 1 + i]);
                start = open + 2 + i;
            }
            _ => {}
        }
    }
    parts.push(&s[start..close]);
    parts
        .into_iter()
        .filter(|p| !p.trim().is_empty())
        .flat_map(|p| expand(&format!("{prefix}{}", p.trim())))
        .collect()
}

fn use_paths(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut rest = text;
    while let Some(i) = rest.find("use ") {
        let b = rest.as_bytes();
        let before_ok = i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_');
        let after = &rest[i + 4..];
        let end = after.find(';').unwrap_or(after.len());
        if before_ok {
            let stmt = after[..end].replace(['\n', '\r'], " ");
            for p in expand(&stmt) {
                out.insert(p.replace(' ', ""));
            }
        }
        rest = &after[end..];
    }
    out
}

fn grammar(text: &str, d: &mut LangData) {
    for line in text.lines() {
        let Some(c) = line.chars().next() else {
            continue;
        };
        if !c.is_ascii_lowercase() {
            continue;
        }
        let word = line.split_whitespace().next().unwrap_or("");
        d.lines.insert(word.to_string());
        d.glines
            .push(line.split_whitespace().collect::<Vec<_>>().join(" "));
        if word == "node" || word == "token" {
            let head = line.split(" = ").next().unwrap_or(line);
            for w in head.split_whitespace().filter(|w| w.starts_with('#')) {
                d.tags.insert(w.trim_start_matches('#').to_string());
            }
        }
    }
}

fn rows(text: &str, d: &mut LangData) {
    let mut section = String::new();
    for line in text.lines().map(str::trim) {
        if let Some(n) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = n.to_string();
            d.sections.insert(section.clone());
            continue;
        }
        if line.is_empty()
            || line.starts_with('#')
            || section.is_empty()
            || line.starts_with("set ")
        {
            continue;
        }
        let b = line.as_bytes();
        for (i, c) in line.char_indices() {
            if c != '=' {
                continue;
            }
            let mut j = i;
            if j > 0 && b[j - 1] == b'!' {
                j -= 1;
            }
            while j > 0 && b[j - 1] == b' ' {
                j -= 1;
            }
            let end = j;
            while j > 0 && (b[j - 1].is_ascii_lowercase() || matches!(b[j - 1], b'_' | b'.')) {
                j -= 1;
            }
            if j < end {
                d.keys.insert(format!("{section}.{}", &line[j..end]));
            }
        }
    }
}

fn options(text: &str, d: &mut LangData) {
    let mut inside = false;
    for line in text.lines() {
        if !line.starts_with(' ') && !line.starts_with('-') {
            inside = line.starts_with("options:");
            continue;
        }
        if inside && let Some(rest) = line.trim_start().strip_prefix("- ") {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_lowercase() || *c == '_' || c.is_ascii_digit())
                .collect();
            if !name.is_empty() {
                d.options.insert(name);
            }
        }
    }
}

fn crate_of(id: &str) -> &str {
    match id {
        "scss" | "less" => "css",
        "vue" | "svelte" | "angular" => "html",
        other => other,
    }
}

impl Langs {
    pub fn load(root: &Path) -> Result<Langs> {
        let mut langs = BTreeMap::new();
        for e in std::fs::read_dir(root.join("crates"))? {
            let dir = e?.path();
            let name = dir.file_name().unwrap().to_string_lossy().to_string();
            let Some(lang) = name.strip_prefix("lang-") else {
                continue;
            };
            let mut d = LangData::default();
            grammar(
                &read(&root.join(format!("grammars/{lang}.grammar"))),
                &mut d,
            );
            rows(&read(&dir.join("format.rules")), &mut d);
            if let Ok(rd) = std::fs::read_dir(&dir) {
                d.files.extend(
                    rd.flatten()
                        .map(|f| f.file_name().to_string_lossy().to_string()),
                );
            }
            let yaml = read(&dir.join("lang.yaml"));
            options(&yaml, &mut d);
            d.rust.push_str(&yaml);
            if let Ok(rd) = std::fs::read_dir(dir.join("rules")) {
                for f in rd.flatten() {
                    for l in read(&f.path()).lines() {
                        if let Some(v) = l.strip_prefix("family:") {
                            d.families.insert(v.trim().trim_matches('"').to_string());
                        }
                        d.pattern_rules |= l.starts_with("rule:");
                    }
                }
            }
            let mut files = Vec::new();
            rs_files(&dir.join("src"), &mut files);
            for f in files {
                let t = read(&f);
                d.uses.extend(use_paths(&t));
                d.rust.push_str(&t);
            }
            langs.insert(lang.to_string(), d);
        }
        let mut rule_langs: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        if let Ok(text) = std::fs::read_to_string(root.join("docs/ai/rules/rules.json")) {
            let v: serde_json::Value = serde_json::from_str(&text)?;
            for r in v["rules"].as_array().into_iter().flatten() {
                let id = r["id"].as_str().unwrap_or_default().to_string();
                let set = rule_langs.entry(id).or_default();
                for l in r["languages"].as_array().into_iter().flatten() {
                    let l = l.as_str().unwrap_or_default();
                    if l == "all" {
                        set.extend(langs.keys().cloned());
                    } else {
                        set.insert(crate_of(l).to_string());
                    }
                }
            }
        }
        Ok(Langs { langs, rule_langs })
    }

    /// The languages whose data sets what `reads` names, or whose code names one of `paths`.
    pub fn users(&self, reads: &[String], mods: &[&Module]) -> BTreeSet<Lang> {
        let mut out = BTreeSet::new();
        for (lang, d) in &self.langs {
            let by_data = reads.iter().any(|r| {
                let Some((k, v)) = r.split_once(':') else {
                    return false;
                };
                let v = v.trim();
                match k {
                    "grammar" => match v.strip_suffix('*') {
                        Some(prefix) => d.glines.iter().any(|l| l.starts_with(prefix)),
                        None => d
                            .glines
                            .iter()
                            .any(|l| l == v || l.starts_with(&format!("{v} "))),
                    },
                    "rulefiles" => v == "pattern" && d.pattern_rules,
                    "rules" if v.contains('.') => d.keys.contains(v),
                    "rules" => d.sections.contains(v),
                    "tag" => d.tags.contains(v.trim_start_matches('#')),
                    "option" => d.options.contains(v),
                    "family" => d.families.contains(v),
                    "file" => d.files.contains(v),
                    "rule" => {
                        self.rule_langs.get(v).is_some_and(|s| s.contains(lang))
                            && !d.rust.contains(&format!("id: \"{v}\""))
                    }
                    "path" => d.rust.contains(v),
                    _ => false,
                }
            });
            let by_use = mods.iter().any(|m| {
                let p = &m.rust_path;
                let parent = p.rsplit_once("::").map_or("", |x| x.0);
                let krate = p.split("::").next().unwrap_or("");
                let two = p.rsplit("::").take(2).collect::<Vec<_>>();
                let last_two = if two.len() == 2 {
                    format!("{}::{}", two[1], two[0])
                } else {
                    p.clone()
                };
                d.uses
                    .iter()
                    .any(|u| u == p || u.starts_with(&format!("{p}::")))
                    || d.rust.contains(&format!("{p}::"))
                    || d.rust.contains(&format!("{last_two}::"))
                    || m.items.iter().any(|i| {
                        d.uses.iter().any(|u| {
                            u.rsplit_once("::").is_some_and(|(pre, last)| {
                                last == i && (pre == parent || pre == krate)
                            })
                        }) || d.rust.contains(&format!("{parent}::{i}"))
                            || d.rust.contains(&format!("{krate}::{i}"))
                    })
            });
            if by_data || by_use {
                out.insert(lang.clone());
            }
        }
        out
    }

    /// Every data item the languages state, with its users: `grammar-line:`, `rules-section:`,
    /// `row-key:`, and `tag:` for the tags shared code reads.
    pub fn data_items(&self, root: &Path) -> Result<BTreeMap<String, BTreeSet<Lang>>> {
        let blob = shared_text(root);
        let reads = tag_reads(&blob);
        let mut out: BTreeMap<String, BTreeSet<Lang>> = BTreeMap::new();
        for (lang, d) in &self.langs {
            let mut add = |k: String| {
                out.entry(k).or_default().insert(lang.clone());
            };
            d.lines
                .iter()
                .for_each(|l| add(format!("grammar-line:{l}")));
            d.sections
                .iter()
                .for_each(|l| add(format!("rules-section:{l}")));
            d.keys.iter().for_each(|l| add(format!("row-key:{l}")));
            for t in &d.tags {
                if reads.contains(t.as_str())
                    || mentions(&blob, &format!("#{t}"))
                    || mentions(&blob, &format!("role: {t}"))
                {
                    add(format!("tag:{t}"));
                }
            }
        }
        Ok(out)
    }
}

/// The tag names the shared code asks about: the string that follows `tag` on a line (`has_tag(k,
/// "x")`, `tagged(n, "x")`; after `tags` all strings to the end of the line). A slot, option or
/// field of the same name is not a read of the tag.
fn tag_reads(text: &str) -> BTreeSet<&str> {
    let mut out = BTreeSet::new();
    for line in text.lines() {
        for (at, _) in line.match_indices("tag") {
            let rest = &line[at..];
            let mut parts = rest.split('"');
            parts.next();
            if rest.starts_with("tags") {
                out.extend(parts.step_by(2));
            } else if let Some(first) = parts.next() {
                out.insert(first);
            }
        }
    }
    out
}

/// Does `text` hold `word` as a whole name (`#modifier` is not in `#modifiers` or `#modifier-list`)?
fn mentions(text: &str, word: &str) -> bool {
    text.match_indices(word).any(|(i, _)| {
        !text[i + word.len()..]
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    })
}

/// The text of the shared crates' sources and shared pattern files.
fn shared_text(root: &Path) -> String {
    let mut s = String::new();
    let Ok(rd) = std::fs::read_dir(root.join("crates")) else {
        return s;
    };
    for e in rd.flatten() {
        let n = e.file_name().to_string_lossy().to_string();
        if n.starts_with("core-") || n == "provider-api" {
            let mut files = Vec::new();
            rs_files(&e.path().join("src"), &mut files);
            // Row attribute words are not grammar tags.
            for f in files.iter().filter(|f| !f.ends_with("scopes/rows.rs")) {
                s.push_str(&read(f));
            }
        }
    }
    if let Ok(rd) = std::fs::read_dir(root.join("crates/core-patterns/shared")) {
        for f in rd.flatten() {
            s.push_str(&read(&f.path()));
        }
    }
    s
}

/// The shared pattern rules: id to (path, first sentence of the description).
pub fn shared_patterns(root: &Path) -> Result<BTreeMap<String, (String, String)>> {
    let mut out = BTreeMap::new();
    let Ok(rd) = std::fs::read_dir(root.join("crates/core-patterns/shared")) else {
        return Ok(out);
    };
    for f in rd.flatten() {
        let p = f.path();
        if p.extension().is_none_or(|x| x != "yaml") {
            continue;
        }
        let id = p.file_stem().unwrap().to_string_lossy().to_string();
        let text = read(&p);
        let mut desc = String::new();
        let mut on = false;
        for l in text.lines() {
            if let Some(v) = l.strip_prefix("description:") {
                on = true;
                desc.push_str(v.trim().trim_start_matches(">-").trim());
                desc.push(' ');
            } else if on && l.starts_with(' ') {
                desc.push_str(l.trim());
                desc.push(' ');
            } else if on {
                break;
            }
        }
        let first = desc
            .split(". ")
            .next()
            .unwrap_or("")
            .trim()
            .trim_end_matches('.')
            .replace('|', "/");
        out.insert(
            id,
            (
                p.strip_prefix(root)?.to_string_lossy().replace('\\', "/"),
                first.chars().take(180).collect(),
            ),
        );
    }
    Ok(out)
}

pub struct Catalogs(Vec<(String, String, Vec<String>)>);

pub fn catalogs(root: &Path) -> Result<Catalogs> {
    let mut out = Vec::new();
    let mut files: Vec<_> = std::fs::read_dir(root.join("docs/ai/parity"))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "yaml"))
        .collect();
    files.sort();
    for p in files {
        let text = read(&p);
        let mut tool = String::new();
        let mut entries = false;
        for l in text.lines() {
            if let Some(v) = l.strip_prefix("tool:") {
                tool = v.trim().trim_matches('"').to_string();
            } else if l.starts_with("entries:") {
                entries = true;
            } else if entries && l.starts_with("  ") && !l.starts_with("   ") {
                let Some((k, v)) = l.trim().split_once(": ") else {
                    continue;
                };
                let v = v.trim().trim_matches('"');
                if let Some(r) = v.strip_prefix("rule ") {
                    let ids = r
                        .split('|')
                        .next()
                        .unwrap_or("")
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    out.push((tool.clone(), k.trim_matches('"').to_string(), ids));
                }
            } else if entries && !l.starts_with(' ') && !l.is_empty() {
                entries = false;
            }
        }
    }
    Ok(Catalogs(out))
}

impl Catalogs {
    /// `tool entry, entry` strings for the catalog entries that map to any of `ids`.
    pub fn tools_of(&self, ids: &[String]) -> Vec<String> {
        let mut by_tool: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for (tool, entry, rules) in &self.0 {
            if rules.iter().any(|r| ids.contains(r)) {
                by_tool.entry(tool).or_default().push(entry);
            }
        }
        by_tool
            .into_iter()
            .map(|(t, e)| format!("{t} {}", e.join(", ")))
            .collect()
    }
}
