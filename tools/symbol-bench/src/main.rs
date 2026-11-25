//! `symbol-bench DIR LANG [PASSES]`: the symbol extraction (`core_semantics::symbols`) over
//! every file of a language under DIR. Parsing is outside the timer; the best of PASSES passes
//! (default 3) of the extraction alone is reported with the files' counts, the share of declarations
//! the index must answer `Unknown` for, and the memory the symbols of all files keep (a count of
//! their payload and the resident set before and after).
//!
//! `symbol-bench index DIR LANG [FILES]`: the index of a project of FILES files of DIR (default
//! 6,000, spread over the whole set): the first answer, then the time from an edit to the answer
//! after a body edit and after a signature edit, with the memory of the whole index.
mod index;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static A: mimalloc::MiMalloc = mimalloc::MiMalloc;

use anyhow::{Result, anyhow};
use core_semantics::symbols::{FileSymbols, extract};
use perf_bench::file;
use std::path::Path;
use std::time::{Duration, Instant};

/// Bytes the symbols of one file hold: structs, strings and the vectors' capacity.
fn payload(s: &FileSymbols) -> usize {
    use core_semantics::symbols::{ConstSymbol, FileImport, FunctionSymbol, Member, TypeSymbol};
    let name = |n: &core_semantics::symbols::Name| {
        std::mem::size_of::<Box<[Box<str>]>>() + n.iter().map(|s| s.len() + 16).sum::<usize>()
    };
    let mut bytes = std::mem::size_of::<FileSymbols>();
    bytes += s.types.capacity() * std::mem::size_of::<TypeSymbol>();
    bytes += s.functions.capacity() * std::mem::size_of::<FunctionSymbol>();
    bytes += s.constants.capacity() * std::mem::size_of::<ConstSymbol>();
    bytes += s.imports.capacity() * std::mem::size_of::<FileImport>();
    for t in &s.types {
        bytes += t.name.len();
        bytes += t.members.capacity() * std::mem::size_of::<Member>();
        bytes += t.parents.iter().map(|p| name(&p.name)).sum::<usize>();
        bytes += t.mixins.iter().map(name).sum::<usize>();
        for m in &t.members {
            bytes += m.name.len() + m.refs.iter().map(name).sum::<usize>();
        }
    }
    bytes += s.functions.iter().map(|f| f.name.len()).sum::<usize>();
    bytes += s.constants.iter().map(|c| c.name.len()).sum::<usize>();
    bytes += s.imports.iter().map(|i| i.path.len()).sum::<usize>();
    bytes
}

/// Resident memory in bytes (Linux), 0 elsewhere.
pub(crate) fn rss() -> usize {
    std::fs::read_to_string("/proc/self/statm")
        .ok()
        .and_then(|s| s.split_whitespace().nth(1)?.parse::<usize>().ok())
        .map_or(0, |pages| pages * 4096)
}

fn run(dir: &Path, lang: &str, passes: usize) -> Result<()> {
    let providers = languages::providers();
    let mut paths = Vec::new();
    walk(dir, &mut paths);
    let mut inputs = Vec::new();
    for path in paths {
        let Some((i, _)) = provider_api::select_provider_index(&providers, &path) else {
            continue;
        };
        if providers[i].language() != lang {
            continue;
        }
        if let Ok(text) = std::fs::read_to_string(&path) {
            inputs.push((i, path, text));
        }
    }
    let bytes: usize = inputs.iter().map(|(_, _, t)| t.len()).sum();
    let (mut best, mut kept, mut broken) = (Duration::MAX, Vec::new(), 0usize);
    let before = rss();
    let mut after = 0;
    for pass in 0..passes.max(1) {
        let mut spent = Duration::ZERO;
        let mut symbols = Vec::with_capacity(inputs.len());
        broken = 0;
        for (i, path, text) in &inputs {
            let p = providers[*i].as_ref();
            let Some(lang) = p.symbols() else {
                return Err(anyhow!("{} has no symbols.rows", p.language()));
            };
            let (f, _) = file(p, &path.to_string_lossy(), 1, text);
            let Ok(parsed) = p.parse(&f) else {
                broken += 1;
                continue;
            };
            if !parsed.errors.is_empty() {
                broken += 1;
                continue;
            }
            let root = parsed.tree;
            let grammar = p.grammar();
            let table = lang.table(grammar);
            let start = Instant::now();
            let s = extract(&root, grammar, table);
            spent += start.elapsed();
            symbols.push(s);
        }
        if pass == 0 {
            after = rss();
        }
        best = best.min(spent);
        kept = symbols;
    }
    let (mut types, mut members, mut functions, mut constants, mut imports) = (0, 0, 0, 0, 0);
    let (mut conditional, mut incomplete, mut dynamic, mut adapted, mut files_calls) =
        (0, 0, 0, 0, 0);
    let mut held = 0;
    for s in &kept {
        held += payload(s);
        types += s.types.len();
        functions += s.functions.len();
        constants += s.constants.len();
        imports += s.imports.len();
        files_calls += usize::from(!s.declaring_calls.is_empty());
        for t in &s.types {
            members += t.members.len();
            conditional += usize::from(t.conditional);
            incomplete += usize::from(t.incomplete);
            dynamic += usize::from(t.dynamic);
            adapted += usize::from(t.adapted);
        }
    }
    println!(
        "{lang}: {} files ({broken} with errors), {:.1} MB",
        inputs.len(),
        bytes as f64 / 1e6
    );
    println!(
        "extraction: best of {passes} passes {:.0} ms ({:.0} MB/s of source)",
        best.as_secs_f64() * 1e3,
        bytes as f64 / 1e6 / best.as_secs_f64()
    );
    println!(
        "declared: {types} types ({conditional} conditional, {incomplete} incomplete, {dynamic} dynamic, {adapted} adapted), {members} members, {functions} functions, {constants} constants, {imports} imports; {files_calls} files with declaring calls"
    );
    println!(
        "memory of the symbols of {} files: {:.1} MB payload, {:.1} MB resident growth (first pass)",
        kept.len(),
        held as f64 / 1e6,
        after.saturating_sub(before) as f64 / 1e6
    );
    Ok(())
}

pub(crate) fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.path());
    for e in entries {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else {
            out.push(p);
        }
    }
}

fn main() -> Result<()> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let indexed = args.first().is_some_and(|a| a == "index");
    if indexed {
        args.remove(0);
    }
    let (Some(dir), Some(lang)) = (args.first(), args.get(1)) else {
        return Err(anyhow!("symbol-bench [index] DIR LANG [PASSES|FILES]"));
    };
    if indexed {
        let files = args.get(2).map_or(Ok(6000), |n| n.parse())?;
        return index::run(Path::new(dir), lang, files);
    }
    let passes = args.get(2).map_or(Ok(3), |n| n.parse())?;
    run(Path::new(dir), lang, passes)
}
