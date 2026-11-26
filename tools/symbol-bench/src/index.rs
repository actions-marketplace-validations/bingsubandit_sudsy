//! The editor case of the symbol index: a project of many files, an edit, and the first answer
//! that reads the edited file's symbols (see `main.rs`).
use anyhow::{Result, anyhow};
use core_config::EditorConfig;
use core_incremental::{Answer, Database, File, Index, Project};
use core_semantics::symbols::Name;
use std::path::Path;
use std::time::{Duration, Instant};

fn median(mut v: Vec<Duration>) -> Duration {
    v.sort();
    v[v.len() / 2]
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1e3
}

pub fn run(dir: &Path, lang: &str, count: usize) -> Result<()> {
    let providers = languages::providers();
    let mut paths = Vec::new();
    crate::walk(dir, &mut paths);
    paths.retain(|p| {
        provider_api::select_provider_index(&providers, p)
            .is_some_and(|(i, _)| providers[i].language() == lang)
    });
    if paths.len() < count {
        return Err(anyhow!(
            "{} files of {lang}, {count} asked for",
            paths.len()
        ));
    }
    let step = paths.len() / count;
    let (paths, texts): (Vec<_>, Vec<String>) = paths
        .into_iter()
        .step_by(step)
        .take(count)
        .filter_map(|p| Some((p.clone(), std::fs::read_to_string(&p).ok()?)))
        .unzip();
    let mut db = Database::new(providers);
    let before = crate::rss();
    let files: Vec<File> = paths
        .iter()
        .zip(&texts)
        .map(|(p, t)| File::new(&db, p.clone(), 1, t.clone(), false, EditorConfig::default()))
        .collect();
    let project = Project::new(&db, files.clone(), true, None, 0);
    // A type of the first file that has one, asked for from the last.
    let ask = |db: &Database, name: &Name, from: File| {
        let started = Instant::now();
        let answer = Index::new(db, project).type_(from, name);
        (started.elapsed(), matches!(answer, Answer::Known(_)))
    };
    let target = files
        .iter()
        .enumerate()
        .find_map(|(i, &f)| {
            let s = core_incremental::symbols(&db, f).as_ref().ok()?.as_ref()?;
            let t = s.types.first()?;
            Some((i, Name::from([t.name.clone()])))
        })
        .ok_or_else(|| anyhow!("no type in the files"))?;
    let from = files[files.len() - 1];
    let inputs = crate::rss().saturating_sub(before);
    let (first, known) = ask(&db, &target.1, from);
    println!(
        "{count} {lang} files: first answer {:.0} ms (parse and extract of all files), known {known}",
        ms(first)
    );
    let held = crate::rss().saturating_sub(before);
    println!(
        "memory: {:.1} MB resident for the texts (inputs), {:.1} MB after the first answer (trees, symbols, index)",
        inputs as f64 / 1e6,
        held as f64 / 1e6
    );
    let (again, _) = ask(&db, &target.1, from);
    println!("an answer with nothing edited: {:.3} ms", ms(again));
    let mut body = Vec::new();
    let mut signature = Vec::new();
    let mut unchanged = 0;
    for (n, i) in (0..files.len())
        .step_by(files.len() / 60)
        .take(60)
        .enumerate()
    {
        let (text, file) = (&texts[i], files[i]);
        let Some(open) = text
            .find("function ")
            .and_then(|f| text[f..].find("{\n").map(|o| f + o + 2))
        else {
            continue;
        };
        let version = 2 * n as u64 + 2;
        let edited = format!("{}$__edit = 1;\n{}", &text[..open], &text[open..]);
        db.set_file_text(file, version, edited);
        let (t, known) = ask(&db, &target.1, from);
        let _ = known;
        // The file's own symbols are read by the join: time the answer after the edit.
        body.push(t);
        let Some(close) = text.rfind('}') else {
            continue;
        };
        let edited = format!(
            "{}function __added_{n}() {{}}\n{}",
            &text[..close],
            &text[close..]
        );
        db.set_file_text(file, version + 1, edited);
        let (t, _) = ask(&db, &target.1, from);
        signature.push(t);
        db.set_file_text(file, version + 1000, text.clone());
        unchanged += 1;
        let _ = ask(&db, &target.1, from);
    }
    println!(
        "edit to answer over {unchanged} files: body edit median {:.1} ms (max {:.1}), signature edit median {:.1} ms (max {:.1})",
        ms(median(body.clone())),
        ms(*body.iter().max().unwrap_or(&Duration::ZERO)),
        ms(median(signature.clone())),
        ms(*signature.iter().max().unwrap_or(&Duration::ZERO)),
    );
    Ok(())
}
