//! Repository automation, stage 1: `cargo xtask codegen` writes the glue of the language crates
//! (registration, provider, node mapping, options) and the worker's grammar constants from
//! `lang.yaml` and the grammar files only, so it compiles when that glue is stale. Stage 2
//! (`xtask-full`) builds the languages and writes what derives from them (schema, docs, editor
//! lists). `cargo xtask codegen --check` (and the `codegen_is_fresh` test) fails when either is stale.

mod dupes;
mod generate;
pub mod kit;
pub mod lang_crate;
mod mapping;
mod new_rule;
mod options;
mod parser_imports;
mod registry;
mod rename_rule;
mod rows;

pub use dupes::{Mode as DupesMode, run as dupes};
pub use generate::generate as generate_table;
pub use new_rule::new_rule;
pub use parser_imports::check as parser_imports;
pub use rename_rule::rename_rule;
pub use rows::check as rows;

use anyhow::{Context, Result, ensure};
use core_syntax::{Grammar, codegen};
use std::path::{Path, PathBuf};

pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives in the repo root")
        .to_path_buf()
}

/// The stage 1 files as (path relative to the repo root, contents).
pub fn generate() -> Result<Vec<(PathBuf, String)>> {
    let root = repo_root();
    let mut files: Vec<(PathBuf, String)> = Vec::new();
    let mut ids = Vec::new();
    for entry in std::fs::read_dir(root.join("grammars"))? {
        let path = entry?.path();
        if path.extension().is_none_or(|e| e != "grammar") {
            continue;
        }
        let src = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let grammar =
            Grammar::parse(&src).with_context(|| format!("parsing {}", path.display()))?;
        let problems = grammar.check_roles();
        ensure!(
            problems.is_empty(),
            "{}: roles:\n  {}",
            path.display(),
            problems.join("\n  ")
        );
        if path.file_stem().is_some_and(|n| n == "csharp") {
            files.push((
                "workers/csharp/Sudsy.Worker/Generated/CsharpGrammar.g.cs".into(),
                codegen::csharp_constants(&grammar, "grammars/csharp.grammar")?,
            ));
        }
        ids.push(grammar.language_id);
    }
    let n = ids.len();
    ids.sort();
    ids.dedup();
    ensure!(ids.len() == n, "two grammars share a language id");
    let langs = lang_crate::load(&root)?;
    files.extend(lang_crate::generate(&langs)?);
    files.push(options::ids(&root, &langs)?);
    files.extend(registry::generate(
        &lang_crate::tree_sitter(&langs),
        &lang_crate::generated(&langs),
    )?);
    files.extend(kit_files(&root)?);
    Ok(files)
}

/// The kit index and the single-use list (written only once every module has a header).
pub fn kit_files(root: &Path) -> Result<Vec<(PathBuf, String)>> {
    let kit = kit::build(root)?;
    let mut files = vec![(kit::INDEX.into(), kit::index(&kit))];
    if kit
        .errors
        .iter()
        .all(|e| !e.contains("no `//! concept:` header"))
    {
        files.push((kit::RATCHET.into(), kit::ratchet_file(root, &kit)));
    }
    Ok(files)
}

/// Paths of generated files whose checked-in content differs from fresh output.
pub fn stale(files: Vec<(PathBuf, String)>) -> Vec<PathBuf> {
    let root = repo_root();
    files
        .into_iter()
        .filter(|(rel, content)| {
            std::fs::read_to_string(root.join(rel)).ok().as_deref() != Some(content)
        })
        .map(|(rel, _)| rel)
        .collect()
}

/// Writes the files; returns those that changed on disk.
pub fn write(files: Vec<(PathBuf, String)>) -> Result<Vec<PathBuf>> {
    let root = repo_root();
    let mut changed = Vec::new();
    for (rel, content) in files {
        let path = root.join(&rel);
        if std::fs::read_to_string(&path).ok().as_deref() == Some(&content) {
            continue;
        }
        std::fs::create_dir_all(path.parent().expect("file has a parent"))?;
        std::fs::write(&path, content)?;
        println!("wrote {}", rel.display());
        changed.push(rel);
    }
    Ok(changed)
}
