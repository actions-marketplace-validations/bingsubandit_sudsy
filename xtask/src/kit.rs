//! The kit index: concept headers of every module of the shared and language crates, the
//! generated `docs/ai/kit/index.md`, and the two-language checks (docs/ai/architecture/adding-a-language.md).
//!
//! A module's header is the `//!` block at the top of the file with these lines:
//! `concept: <id>[, <id>...]` (a language module may write `new <id>`; any module `none (<reason>)`),
//! `what:`, `tools:`, optionally `kind:`, `rules:` (lint rule ids) and `reads:` (the data a shared
//! mechanism reads: `rules:<section>[.<key>]`, `grammar:<line>`, `tag:<name>`, `option:<name>`,
//! `rule:<id>`, `family:<name>`, `path:<rust path>`), which gives the languages that use it. A shared module whose `reads:` has only `option:`, `rule:` and `path:` entries may have one user: codegen adds its concept to the single-use list, marked `(shared)`.

use anyhow::{Result, bail};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

mod data;
mod render;

pub use data::Langs;

/// A language crate's name without the prefix (`java` for `lang-java`).
pub type Lang = String;

#[derive(Debug, Default, Clone)]
pub struct Header {
    pub present: bool,
    pub ids: Vec<String>,
    pub new: Vec<String>,
    pub none: Option<String>,
    pub what: String,
    pub tools: String,
    /// `what` and `tools` lines of the form `<id>: text`, for modules that declare several concepts.
    pub per_id: BTreeMap<(String, bool), String>,
    pub kind: Option<String>,
    pub rules: Vec<String>,
    pub reads: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Module {
    /// Path from the repo root, `crates/core-format/src/blank.rs`.
    pub path: String,
    pub krate: String,
    pub shared: bool,
    /// `core_format::blank`.
    pub rust_path: String,
    pub header: Header,
    /// Names of the module's `pub` items (`fn`, `struct`, `enum`, `trait`, `const`, `static`, `type`).
    pub items: Vec<String>,
}

impl Module {
    fn lang(&self) -> Option<&str> {
        self.krate.strip_prefix("lang-")
    }
}

pub struct Concept {
    pub id: String,
    pub kind: String,
    pub modules: Vec<String>,
    pub what: String,
    pub tools: Vec<String>,
    pub shared: bool,
    pub reads: Vec<String>,
    pub rules: Vec<String>,
    pub used_by: BTreeSet<Lang>,
    /// (module path, language) of modules that carry the id without using the shared mechanism.
    pub local: Vec<(String, Lang)>,
}

pub struct Kit {
    pub modules: Vec<Module>,
    pub concepts: BTreeMap<String, Concept>,
    pub errors: Vec<String>,
    /// Data items and their users: `grammar:string`, `rules:blank`, `rowkey:blank.parent`, `tag:call`.
    pub data: BTreeMap<String, BTreeSet<Lang>>,
}

pub fn parse_header(src: &str) -> Header {
    let mut h = Header::default();
    let mut started = false;
    for line in src.lines() {
        let Some(rest) = line.strip_prefix("//!") else {
            if !started && (line.trim().is_empty() || line.starts_with("#![")) {
                continue;
            }
            break;
        };
        started = true;
        let rest = rest.trim();
        let Some((key, val)) = rest.split_once(':') else {
            continue;
        };
        let val = val.trim();
        match key {
            "concept" => {
                h.present = true;
                if val.starts_with("none") {
                    h.none = Some(val.to_string());
                    continue;
                }
                for part in val.split(',').map(str::trim).filter(|p| !p.is_empty()) {
                    match part.strip_prefix("new ") {
                        Some(id) => {
                            h.new.push(id.trim().to_string());
                            h.ids.push(id.trim().to_string());
                        }
                        None => h.ids.push(part.to_string()),
                    }
                }
            }
            "what" | "tools" => {
                let is_what = key == "what";
                match val.split_once(": ") {
                    Some((id, text)) if h.ids.iter().any(|i| i == id) => {
                        h.per_id.insert((id.to_string(), is_what), text.to_string());
                    }
                    _ if is_what => h.what = val.to_string(),
                    _ => h.tools = val.to_string(),
                }
            }
            "kind" => h.kind = Some(val.to_string()),
            "rules" => h.rules = list(val),
            "reads" => h.reads = list(val),
            _ => {}
        }
    }
    h
}

fn list(v: &str) -> Vec<String> {
    v.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Every module of the shared and language crates that needs a header.
pub fn modules(root: &Path) -> Result<Vec<Module>> {
    let mut out = Vec::new();
    let mut crates: Vec<PathBuf> = std::fs::read_dir(root.join("crates"))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    crates.sort();
    for dir in crates {
        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        let shared = name.starts_with("core-") || name == "provider-api";
        if !shared && !name.starts_with("lang-") {
            continue;
        }
        let mut files = Vec::new();
        walk(&dir.join("src"), &mut files);
        files.sort();
        for f in files {
            let rel = f.strip_prefix(root)?.to_string_lossy().replace('\\', "/");
            let src = std::fs::read_to_string(&f)?;
            let inner = rel.split("/src/").nth(1).unwrap_or("");
            out.push(Module {
                path: rel.clone(),
                rust_path: rust_path(&name, inner),
                krate: name.clone(),
                shared,
                header: parse_header(&src),
                items: pub_items(&src),
            });
        }
    }
    Ok(out)
}

fn pub_items(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    for l in src.lines() {
        let Some(rest) = l.strip_prefix("pub ") else {
            continue;
        };
        let mut w = rest.split_whitespace();
        let kw = w.next().unwrap_or("");
        if matches!(
            kw,
            "fn" | "struct" | "enum" | "trait" | "const" | "static" | "type"
        ) && let Some(name) = w.next()
        {
            let name: String = name
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if name.len() >= 6 {
                out.push(name);
            }
        }
    }
    out
}

fn rust_path(krate: &str, inner: &str) -> String {
    let mut p = vec![krate.replace('-', "_")];
    for seg in inner.trim_end_matches(".rs").split('/') {
        if !matches!(seg, "lib" | "mod" | "main") {
            p.push(seg.to_string());
        }
    }
    p.join("::")
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if p.is_dir() {
            if name != "tests" {
                walk(&p, out);
            }
        } else if name.ends_with(".rs")
            && !name.contains("generated")
            && name != "tests.rs"
            && !name.ends_with("_tests.rs")
        {
            out.push(p);
        }
    }
}

/// The kind of a concept from where its module lives.
fn default_kind(m: &Module) -> &'static str {
    if m.shared {
        return match m.krate.as_str() {
            "core-format" => "format",
            "core-rules" | "core-patterns" => "lint",
            "core-semantics" | "core-flow" => "semantics",
            "core-syntax" | "core-treesitter" | "core-incremental" | "provider-api" => "parse",
            _ => "config",
        };
    }
    let p = &m.path;
    if p.contains("/format") || p.contains("/layout") {
        "format"
    } else if p.contains("/rules") || p.contains("/checks") || p.contains("/analysis") {
        "lint"
    } else if p.contains("/semantic") || p.contains("/scope") {
        "semantics"
    } else if p.contains("/parse") || p.contains("/lexer") || p.contains("/adapter") {
        "parse"
    } else {
        "other"
    }
}

/// Builds the index: concepts, their users and the data-item users, plus every error found.
pub fn build(root: &Path) -> Result<Kit> {
    let modules = modules(root)?;
    let langs = Langs::load(root)?;
    let catalogs = data::catalogs(root)?;
    let mut errors = Vec::new();
    let mut concepts: BTreeMap<String, Concept> = BTreeMap::new();

    for m in &modules {
        if !m.header.present {
            errors.push(format!("{}: no `//! concept:` header", m.path));
        } else if m.header.what.is_empty()
            && m.header.per_id.is_empty()
            && m.header.none.is_none()
            && (m.shared || !m.header.new.is_empty())
        {
            errors.push(format!("{}: header needs a `//! what:` line", m.path));
        }
        if m.shared && !m.header.new.is_empty() {
            errors.push(format!("{}: `new` is for language modules", m.path));
        }
        let owns = m.shared || !m.header.new.is_empty();
        if !owns {
            continue;
        }
        for id in &m.header.ids {
            if !m.shared && !m.header.new.contains(id) {
                continue;
            }
            let c = concepts.entry(id.clone()).or_insert_with(|| Concept {
                id: id.clone(),
                kind: m
                    .header
                    .kind
                    .clone()
                    .unwrap_or_else(|| default_kind(m).to_string()),
                modules: Vec::new(),
                what: m
                    .header
                    .per_id
                    .get(&(id.clone(), true))
                    .unwrap_or(&m.header.what)
                    .clone(),
                tools: Vec::new(),
                shared: m.shared,
                reads: Vec::new(),
                rules: Vec::new(),
                used_by: BTreeSet::new(),
                local: Vec::new(),
            });
            if m.shared != c.shared {
                errors.push(format!(
                    "{}: concept `{id}` is declared by shared and language modules",
                    m.path
                ));
            }
            c.modules.push(m.path.clone());
            let tools = m
                .header
                .per_id
                .get(&(id.clone(), false))
                .unwrap_or(&m.header.tools);
            if !tools.is_empty() && !c.tools.contains(tools) {
                c.tools.push(tools.clone());
            }
            c.reads.extend(m.header.reads.iter().cloned());
            c.rules.extend(m.header.rules.iter().cloned());
        }
    }
    for (id, what) in data::shared_patterns(root)? {
        let (path, what) = what;
        concepts.entry(id.clone()).or_insert_with(|| Concept {
            id: id.clone(),
            kind: "lint".into(),
            modules: vec![path],
            what,
            tools: Vec::new(),
            shared: true,
            reads: vec![format!("rule:{id}")],
            rules: vec![id.clone()],
            used_by: BTreeSet::new(),
            local: Vec::new(),
        });
    }

    // Users of the shared concepts.
    for c in concepts.values_mut() {
        if !c.shared {
            continue;
        }
        let mods: Vec<&Module> = c
            .modules
            .iter()
            .filter_map(|p| modules.iter().find(|m| &m.path == p))
            .collect();
        c.used_by = langs.users(&c.reads, &mods);
    }
    // Language modules carrying a concept: users of `new` concepts, LOCAL markers elsewhere.
    let mut carriers: BTreeMap<String, Vec<(&Module, Lang)>> = BTreeMap::new();
    for m in modules.iter().filter(|m| !m.shared) {
        let lang = m.lang().unwrap_or_default().to_string();
        for id in &m.header.ids {
            carriers
                .entry(id.clone())
                .or_default()
                .push((m, lang.clone()));
        }
    }
    for (id, list) in &carriers {
        let Some(c) = concepts.get_mut(id) else {
            for (m, _) in list {
                errors.push(format!("{}: concept `{id}` is not in the index (declare `concept: new {id}` with a `what:` line, or use an existing id)", m.path));
            }
            continue;
        };
        if c.shared {
            for (m, lang) in list {
                if !c.used_by.contains(lang) {
                    c.local.push((m.path.clone(), lang.clone()));
                }
            }
        } else {
            let langs: BTreeSet<&Lang> = list.iter().map(|(_, l)| l).collect();
            c.used_by = langs.iter().map(|l| (*l).clone()).collect();
            if langs.len() > 1 {
                for (m, lang) in list {
                    c.local.push((m.path.clone(), lang.clone()));
                }
            }
        }
    }
    for c in concepts.values_mut() {
        c.modules.sort();
        c.modules.dedup();
        c.reads.sort();
        c.reads.dedup();
        c.rules.sort();
        c.rules.dedup();
        c.local.sort();
        c.local.dedup();
        let mut ids = c.rules.clone();
        if c.kind == "lint" {
            ids.push(c.id.clone());
        }
        for t in catalogs.tools_of(&ids) {
            if !c.tools.contains(&t) {
                c.tools.push(t);
            }
        }
    }
    errors.sort();
    Ok(Kit {
        modules,
        concepts,
        errors,
        data: langs.data_items(root)?,
    })
}

/// `docs/ai/kit/index.md`.
pub fn index(kit: &Kit) -> String {
    render::index(kit)
}

/// The ratchet entries: single-use data items and concepts, and LOCAL markers, each with a note.
pub fn single_use(kit: &Kit) -> BTreeMap<String, String> {
    render::single_use(kit)
}

pub fn check_ratchet(kit: &Kit, baseline: &str) -> Vec<String> {
    render::check_ratchet(kit, baseline)
}

/// The ratchet file to write: today's list when there is none, else the baseline minus what no longer holds.
pub fn ratchet_file(root: &Path, kit: &Kit) -> String {
    match std::fs::read_to_string(root.join(RATCHET)) {
        Ok(base) => render::shrink(kit, &base),
        Err(_) => render::ratchet_text(kit),
    }
}

pub const RATCHET: &str = "docs/ai/design/single-use.txt";
pub const INDEX: &str = "docs/ai/kit/index.md";

/// Fails with every problem of the headers, the index and the ratchet.
pub fn check(root: &Path) -> Result<()> {
    let kit = build(root)?;
    let mut errors = kit.errors.clone();
    match std::fs::read_to_string(root.join(RATCHET)) {
        Ok(base) => errors.extend(check_ratchet(&kit, &base)),
        Err(_) => errors.push(format!("{RATCHET} is missing (run `cargo xtask codegen`)")),
    }
    if errors.is_empty() {
        return Ok(());
    }
    let n = errors.len();
    errors.truncate(60);
    bail!("kit: {n} problems:\n  {}", errors.join("\n  "))
}
