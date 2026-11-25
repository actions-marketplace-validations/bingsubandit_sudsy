//! The editor case of the rules that read the symbol index: every file linted once with the rules
//! on, then edits, timing the lint of the edited file and of a file that did not change (see
//! `main.rs`).
use anyhow::{Result, anyhow};
use core_config::{ConfigChain, EditorConfig};
use core_incremental::{Database, File, lint, open_projects};
use std::path::Path;
use std::time::{Duration, Instant};

fn median(mut v: Vec<Duration>) -> Duration {
    v.sort();
    v[v.len() / 2]
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1e3
}

/// A declaration the language reads as a new symbol, appended to a file.
fn addition(lang: &str, n: usize) -> String {
    match lang {
        "php" => format!("\nfunction __added_{n}() {{}}\n"),
        _ => format!("\nmessage Added{n} {{}}\n"),
    }
}

pub fn run(dir: &Path, lang: &str, rules: &str) -> Result<()> {
    let providers = languages::providers();
    let mut paths = Vec::new();
    crate::walk(dir, &mut paths);
    paths.retain(|p| {
        provider_api::select_provider_index(&providers, p)
            .is_some_and(|(i, _)| providers[i].language() == lang)
    });
    let mut db = Database::new(providers);
    let yaml: String = rules
        .split(',')
        .map(|r| format!("{r}: warn\n"))
        .map(|l| format!("    {l}"))
        .collect();
    db.set_config(ConfigChain::parse(&format!("lint:\n  rules:\n{yaml}")));
    let before = crate::rss();
    let mut files: Vec<File> = Vec::new();
    let mut texts: Vec<String> = Vec::new();
    for p in &paths {
        let Ok(text) = std::fs::read_to_string(p) else {
            continue;
        };
        files.push(File::new(
            &db,
            p.clone(),
            1,
            text.clone(),
            false,
            EditorConfig::default(),
        ));
        texts.push(text);
    }
    if files.is_empty() {
        return Err(anyhow!("no {lang} files in {}", dir.display()));
    }
    let root = std::path::absolute(dir)?;
    let started = Instant::now();
    open_projects(&mut db, &files, &[root]);
    let opened = started.elapsed();
    let inputs = crate::rss().saturating_sub(before);
    let started = Instant::now();
    let mut found = 0;
    for &f in &files {
        let linted = lint(&db, f);
        found += linted.as_ref().map_or(0, |l| {
            l.diagnostics
                .iter()
                .filter(|d| rules.split(',').any(|r| r == &*d.rule))
                .count()
        });
    }
    let first = started.elapsed();
    println!(
        "{} {lang} files, rules {rules}: projects opened in {:.0} ms, first lint of all {:.0} ms, {found} findings",
        files.len(),
        ms(opened),
        ms(first)
    );
    println!(
        "memory: {:.1} MB resident for the texts, {:.1} MB after the first lint",
        inputs as f64 / 1e6,
        crate::rss().saturating_sub(before) as f64 / 1e6
    );
    let (mut own, mut other, mut again) = (Vec::new(), Vec::new(), Vec::new());
    let step = (files.len() / 40).max(1);
    for (n, i) in (0..files.len()).step_by(step).take(40).enumerate() {
        let (file, text) = (files[i], &texts[i]);
        let reader = files[(i + 1) % files.len()];
        db.set_file_text(
            file,
            2 * n as u64 + 2,
            format!("{text}{}", addition(lang, n)),
        );
        let started = Instant::now();
        let _ = lint(&db, file);
        own.push(started.elapsed());
        let started = Instant::now();
        let _ = lint(&db, reader);
        other.push(started.elapsed());
        db.set_file_text(file, 2 * n as u64 + 3, text.clone());
        let started = Instant::now();
        let _ = lint(&db, file);
        again.push(started.elapsed());
    }
    println!(
        "after an edit adding a symbol, over {} files: lint of the edited file median {:.1} ms (max {:.1}), of the next file median {:.1} ms (max {:.1}); the edit undone: median {:.1} ms",
        own.len(),
        ms(median(own.clone())),
        ms(*own.iter().max().unwrap_or(&Duration::ZERO)),
        ms(median(other.clone())),
        ms(*other.iter().max().unwrap_or(&Duration::ZERO)),
        ms(median(again)),
    );
    Ok(())
}
