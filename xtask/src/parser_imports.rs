//! Parser crates (the per-language upstream parsers) may be named only in a language crate's
//! `adapter.rs`, its `adapter/` folder or a generated file; `xtask/parser-imports.txt` lists the
//! files that still do and may only shrink: a new import outside an adapter and an entry that no
//! longer imports both fail `cargo xtask codegen --check`.

use anyhow::{Context, Result, bail};
use std::collections::BTreeSet;
use std::path::Path;

/// A parser crate is a dependency of a `lang-*` crate whose name starts with one of the file's
/// `prefix` lines; `allow` lines are the files (relative to the repo root) that may import one.
pub struct Config {
    prefixes: Vec<String>,
    allow: BTreeSet<String>,
}

impl Config {
    pub fn parse(text: &str) -> Self {
        let (mut prefixes, mut allow) = (Vec::new(), BTreeSet::new());
        for line in text.lines().map(str::trim) {
            match line.split_once(' ') {
                Some(("prefix", p)) => prefixes.push(p.trim().to_string()),
                Some(("allow", f)) => {
                    allow.insert(f.trim().to_string());
                }
                _ => {}
            }
        }
        Config { prefixes, allow }
    }
}

/// The names (Rust spelling) of the parser crates among the dependencies of one `Cargo.toml`.
fn parser_deps(manifest: &str, prefixes: &[String]) -> Vec<String> {
    let mut in_deps = false;
    let mut out = Vec::new();
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            in_deps = line.ends_with("dependencies]");
        } else if in_deps {
            let name: String = line
                .chars()
                .take_while(|c| c.is_alphanumeric() || matches!(c, '-' | '_'))
                .collect();
            if prefixes.iter().any(|p| name.starts_with(p.as_str())) {
                out.push(name.replace('-', "_"));
            }
        }
    }
    out
}

fn names(source: &str, crate_name: &str) -> bool {
    let is_ident = |c: char| c.is_alphanumeric() || c == '_';
    source.lines().any(|line| {
        let code = line.split("//").next().unwrap_or("");
        code.match_indices(crate_name).any(|(i, m)| {
            !code[..i].ends_with(is_ident) && !code[i + m.len()..].starts_with(is_ident)
        })
    })
}

fn is_adapter(path: &str) -> bool {
    let mut parts = path.split('/').skip(2);
    parts.any(|p| p == "adapter" || p == "adapter.rs") || path.ends_with(".generated.rs")
}

/// The problems of the language crates' sources, as (path, text) files with paths relative to the
/// repo root, against the parser crates and the allowlist.
fn problems(files: &[(String, String)], parsers: &[String], config: &Config) -> Vec<String> {
    let mut out = Vec::new();
    let mut importing = BTreeSet::new();
    for (path, source) in files {
        if is_adapter(path) {
            continue;
        }
        if let Some(p) = parsers.iter().find(|p| names(source, p)) {
            importing.insert(path.as_str());
            if !config.allow.contains(path) {
                out.push(format!(
                    "{path} imports {p} outside an adapter (move it into the language's adapter)"
                ));
            }
        }
    }
    for path in config
        .allow
        .iter()
        .filter(|a| !importing.contains(a.as_str()))
    {
        out.push(format!(
            "{path} no longer imports a parser crate: remove it from xtask/parser-imports.txt"
        ));
    }
    out
}

fn rust_files(dir: &Path, root: &Path, out: &mut Vec<(String, String)>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            rust_files(&path, root, out)?;
        } else if path.extension().is_some_and(|e| e == "rs") {
            let rel = path
                .strip_prefix(root)?
                .to_string_lossy()
                .replace('\\', "/");
            out.push((rel, std::fs::read_to_string(&path)?));
        }
    }
    Ok(())
}

pub fn check(root: &Path) -> Result<()> {
    let config = Config::parse(
        &std::fs::read_to_string(root.join("xtask/parser-imports.txt"))
            .context("reading xtask/parser-imports.txt")?,
    );
    let (mut parsers, mut files) = (Vec::new(), Vec::new());
    for entry in std::fs::read_dir(root.join("crates"))? {
        let dir = entry?.path();
        if !dir
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with("lang-"))
        {
            continue;
        }
        parsers.extend(parser_deps(
            &std::fs::read_to_string(dir.join("Cargo.toml"))?,
            &config.prefixes,
        ));
        rust_files(&dir, root, &mut files)?;
    }
    let problems = problems(&files, &parsers, &config);
    if !problems.is_empty() {
        bail!("parser crates outside adapters:\n{}", problems.join("\n"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(files: &[(&str, &str)], allow: &str) -> Vec<String> {
        let files: Vec<_> = files
            .iter()
            .map(|(p, s)| (p.to_string(), s.to_string()))
            .collect();
        let config = Config::parse(&format!("prefix oxc\n{allow}"));
        let parsers = parser_deps(
            "[dependencies]\noxc_ast.workspace = true\nrowan = 1\n",
            &config.prefixes,
        );
        problems(&files, &parsers, &config)
    }

    #[test]
    fn parser_crates_come_from_the_manifest() {
        let p = vec!["oxc".to_string(), "tree-sitter".to_string()];
        let m = "[package]\nname = \"x\"\n[dependencies]\noxc-parser.workspace = true\ntree-sitter-md = { workspace = true }\nrowan.workspace = true\n[features]\noxc-x = []\n";
        assert_eq!(parser_deps(m, &p), ["oxc_parser", "tree_sitter_md"]);
    }

    #[test]
    fn adapters_and_generated_files_may_import() {
        let src = "use oxc_ast::Foo;";
        let files = [
            ("crates/lang-js/src/adapter.rs", src),
            ("crates/lang-js/src/adapter/x.rs", src),
            ("crates/lang-js/src/provider.generated.rs", src),
        ];
        assert!(run(&files, "").is_empty());
    }

    #[test]
    fn a_new_import_outside_an_adapter_fails() {
        let files = [("crates/lang-js/src/rules/new.rs", "use oxc_ast::Foo;")];
        assert_eq!(run(&files, "").len(), 1);
        assert!(run(&files, "allow crates/lang-js/src/rules/new.rs").is_empty());
    }

    #[test]
    fn comments_and_longer_names_do_not_count() {
        let files = [(
            "crates/lang-js/src/a.rs",
            "// oxc_ast::Foo\nuse oxc_ast_visit::Bar;\nlet oxc_ast_x = 1;",
        )];
        assert!(run(&files, "").is_empty());
    }

    #[test]
    fn an_entry_that_no_longer_imports_fails() {
        let files = [("crates/lang-js/src/a.rs", "fn main() {}")];
        let out = run(
            &files,
            "allow crates/lang-js/src/a.rs\nallow crates/lang-js/src/gone.rs",
        );
        assert_eq!(out.len(), 2);
        assert!(out[0].contains("no longer imports"));
    }
}
