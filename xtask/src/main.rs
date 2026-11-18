use anyhow::{Result, bail, ensure};
use std::process::Command;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["codegen"] => {
            xtask::write(xtask::generate()?)?;
            full(&["codegen"])
        }
        ["codegen", "--check"] => {
            xtask::parser_imports(&xtask::repo_root())?;
            // Stage 2 builds the languages, which stale glue would break: report that first.
            let stale = xtask::stale(xtask::generate()?);
            if !stale.is_empty() {
                bail!("stale generated files (run `cargo xtask codegen`): {stale:?}");
            }
            xtask::kit::check(&xtask::repo_root())?;
            xtask::rows(&xtask::repo_root())?;
            full(&["codegen", "--check"])
        }
        ["rows"] => xtask::rows(&xtask::repo_root()),
        ["kit", "--write"] => {
            xtask::write(xtask::kit_files(&xtask::repo_root())?)?;
            Ok(())
        }
        ["kit"] => {
            let kit = xtask::kit::build(&xtask::repo_root())?;
            for e in &kit.errors {
                println!("{e}");
            }
            println!(
                "{} modules, {} concepts, {} problems",
                kit.modules.len(),
                kit.concepts.len(),
                kit.errors.len()
            );
            Ok(())
        }
        ["dupes"] => xtask::dupes(&xtask::repo_root(), xtask::DupesMode::Report),
        ["dupes", "--check"] => xtask::dupes(&xtask::repo_root(), xtask::DupesMode::Check),
        ["dupes", "--baseline"] => xtask::dupes(&xtask::repo_root(), xtask::DupesMode::Baseline),
        ["affected", rest @ ..] => {
            let mut a = vec!["affected"];
            a.extend(rest);
            full(&a)
        }
        ["css-data", dir] => {
            let path = xtask::repo_root().join("crates/lang-css/data/names.txt");
            std::fs::write(path, xtask::css_data(std::path::Path::new(dir))?)?;
            Ok(())
        }
        ["new-rule", language, id] => print_new_rule(language, id, false),
        ["new-rule", language, id, "--rust"] => print_new_rule(language, id, true),
        ["rename-rule", old, new] => rename(old, new, false),
        ["rename-rule", old, new, "--dry-run"] => rename(old, new, true),
        _ => bail!(
            "usage: cargo xtask codegen [--check] | affected [--base REV] [--all] | dupes [--check|--baseline] | new-rule <language> <id> [--rust] | rename-rule <old> <new> [--dry-run] | css-data <npm dir>"
        ),
    }
}

fn print_new_rule(language: &str, id: &str, rust: bool) -> Result<()> {
    println!(
        "{}",
        xtask::new_rule(&xtask::repo_root(), language, id, rust)?
    );
    Ok(())
}

/// Renames a rule id everywhere, then regenerates the tables and the schema (not on a dry run).
fn rename(old: &str, new: &str, dry_run: bool) -> Result<()> {
    for line in xtask::rename_rule(&xtask::repo_root(), old, new, dry_run)? {
        println!("{line}");
    }
    if dry_run {
        return Ok(());
    }
    xtask::write(xtask::generate()?)?;
    full(&["codegen"])?;
    println!(
        "next: CHANGELOG.md (list the rename), presets and crates/cli/src/init mappings the list above left, then scripts/gate.sh"
    );
    Ok(())
}

/// Stage 2 and the commands that need the compiled languages: `xtask-full`, built after stage 1.
fn full(args: &[&str]) -> Result<()> {
    let status = Command::new("cargo")
        .args(["run", "--quiet", "--package", "xtask-full", "--"])
        .args(args)
        .status()?;
    ensure!(status.success(), "xtask-full {args:?} failed");
    Ok(())
}
