//! `cargo xtask dupes`: a token-window similarity scan of the hand-written Rust of the `lang-*` and
//! `core-*` crates. Identifiers and literals are normalized, windows of `WINDOW` tokens are hashed,
//! and a window found in two crates is a duplicate. Report: one line per pair of files of different
//! crates with the number of duplicated tokens. `--check` fails when a pair is new or grew against
//! `docs/ai/design/dupes-baseline.txt`; `--baseline` writes the file (first run) or shrinks it.

use anyhow::{Result, bail};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

const WINDOW: usize = 60;
/// A pair of files must share at least this many tokens to count.
const MIN_TOKENS: usize = 90;
/// A window in more places than this is boilerplate (an enum list, a match table).
const MAX_PLACES: usize = 12;
const BASELINE: &str = "docs/ai/design/dupes-baseline.txt";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Report,
    Check,
    Baseline,
}

struct File {
    path: String,
    krate: String,
    /// (normalized token hash, line)
    tokens: Vec<(u64, u32)>,
}

const KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
    "while", "async", "await", "dyn", "Some", "None", "Ok", "Err",
];

fn hash_str(s: &str) -> u64 {
    s.bytes().fold(0xcbf29ce484222325u64, |h, b| {
        (h ^ b as u64).wrapping_mul(0x100000001b3)
    })
}

/// Normalized tokens of Rust source: comments, `use` items and `#[cfg(test)]` items dropped,
/// identifiers and literals each one token class.
fn tokenize(src: &str) -> Vec<(u64, u32)> {
    let b: Vec<char> = src.chars().collect();
    let mut out: Vec<(u64, u32)> = Vec::new();
    let (mut i, mut line) = (0usize, 1u32);
    let (ident, lit) = (hash_str("<id>"), hash_str("<lit>"));
    while i < b.len() {
        let c = b[i];
        if c == '\n' {
            line += 1;
            i += 1;
        } else if c.is_whitespace() {
            i += 1;
        } else if c == '/' && b.get(i + 1) == Some(&'/') {
            while i < b.len() && b[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && b.get(i + 1) == Some(&'*') {
            let mut depth = 0;
            while i < b.len() {
                if b[i] == '/' && b.get(i + 1) == Some(&'*') {
                    depth += 1;
                    i += 2;
                } else if b[i] == '*' && b.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    if b[i] == '\n' {
                        line += 1;
                    }
                    i += 1;
                }
            }
        } else if c == '"' || (c == 'r' && matches!(b.get(i + 1), Some('"') | Some('#'))) {
            let mut hashes = 0;
            if c == 'r' {
                i += 1;
                while b.get(i) == Some(&'#') {
                    hashes += 1;
                    i += 1;
                }
            }
            i += 1;
            while i < b.len() {
                if c != 'r' && b[i] == '\\' {
                    i += 2;
                    continue;
                }
                if b[i] == '\n' {
                    line += 1;
                }
                if b[i] == '"' && (0..hashes).all(|k| b.get(i + 1 + k) == Some(&'#')) {
                    i += 1 + hashes;
                    break;
                }
                i += 1;
            }
            out.push((lit, line));
        } else if c == '\'' {
            // A character literal or a lifetime.
            if b.get(i + 2) == Some(&'\'') || (b.get(i + 1) == Some(&'\\')) {
                i += 2;
                while i < b.len() && b[i] != '\'' {
                    i += 1;
                }
                i += 1;
                out.push((lit, line));
            } else {
                i += 1;
                while i < b.len() && (b[i].is_alphanumeric() || b[i] == '_') {
                    i += 1;
                }
                out.push((hash_str("'a"), line));
            }
        } else if c.is_ascii_digit() {
            while i < b.len()
                && (b[i].is_alphanumeric() || b[i] == '_' || b[i] == '.')
                && !(b[i] == '.' && b.get(i + 1) == Some(&'.'))
            {
                i += 1;
            }
            out.push((lit, line));
        } else if c.is_alphabetic() || c == '_' {
            let s = i;
            while i < b.len() && (b[i].is_alphanumeric() || b[i] == '_') {
                i += 1;
            }
            let w: String = b[s..i].iter().collect();
            out.push((
                if KEYWORDS.contains(&w.as_str()) {
                    hash_str(&w)
                } else {
                    ident
                },
                line,
            ));
        } else {
            out.push((hash_str(&c.to_string()), line));
            i += 1;
        }
    }
    strip_items(out)
}

/// Drops `use ...;` items and the items after `#[cfg(test)]`.
fn strip_items(t: Vec<(u64, u32)>) -> Vec<(u64, u32)> {
    let (use_kw, semi, hash, lb, rb, lbr, rbr) = (
        hash_str("use"),
        hash_str(";"),
        hash_str("#"),
        hash_str("{"),
        hash_str("}"),
        hash_str("["),
        hash_str("]"),
    );
    let cfg_test = [
        hash,
        lbr,
        hash_str("<id>"),
        hash_str("("),
        hash_str("<id>"),
        hash_str(")"),
        rbr,
    ];
    let mut out = Vec::with_capacity(t.len());
    let mut i = 0;
    while i < t.len() {
        if t[i].0 == use_kw && out.last().is_none_or(|p: &(u64, u32)| p.0 != hash_str(".")) {
            while i < t.len() && t[i].0 != semi {
                i += 1;
            }
            i += 1;
            continue;
        }
        if i + cfg_test.len() < t.len()
            && t[i..i + cfg_test.len()]
                .iter()
                .map(|x| x.0)
                .eq(cfg_test.iter().copied())
        {
            // `#[cfg(test)]` (identifiers are normalized: also skips `#[cfg(feature)]` items, which is fine).
            let mut j = i + cfg_test.len();
            while j < t.len() && t[j].0 != lb && t[j].0 != semi {
                j += 1;
            }
            if j < t.len() && t[j].0 == lb {
                let mut depth = 0;
                while j < t.len() {
                    if t[j].0 == lb {
                        depth += 1;
                    } else if t[j].0 == rb {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    j += 1;
                }
            }
            i = j + 1;
            continue;
        }
        out.push(t[i]);
        i += 1;
    }
    out
}

fn files(root: &Path) -> Result<Vec<File>> {
    let mut out = Vec::new();
    let mut crates: Vec<_> = std::fs::read_dir(root.join("crates"))?
        .flatten()
        .map(|e| e.path())
        .collect();
    crates.sort();
    for dir in crates {
        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        if !(name.starts_with("lang-") || name.starts_with("core-") || name == "provider-api") {
            continue;
        }
        let mut stack = vec![dir.join("src")];
        let mut paths = Vec::new();
        while let Some(d) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&d) else {
                continue;
            };
            for e in rd.flatten() {
                let p = e.path();
                let n = e.file_name().to_string_lossy().to_string();
                if p.is_dir() {
                    if n != "tests" {
                        stack.push(p);
                    }
                } else if n.ends_with(".rs") && !n.contains("generated") && n != "tests.rs" {
                    paths.push(p);
                }
            }
        }
        paths.sort();
        for p in paths {
            let src = std::fs::read_to_string(&p)?;
            out.push(File {
                path: p.strip_prefix(root)?.to_string_lossy().replace('\\', "/"),
                krate: name.clone(),
                tokens: tokenize(&src),
            });
        }
    }
    Ok(out)
}

/// Duplicated tokens per pair of files of different crates, and the line spans of the first pair.
fn scan(files: &[File]) -> BTreeMap<(String, String), usize> {
    let mut places: HashMap<u64, Vec<(u32, u32)>> = HashMap::new();
    let mut window_hashes: Vec<Vec<u64>> = Vec::new();
    for (fi, f) in files.iter().enumerate() {
        let t = &f.tokens;
        let mut hs = Vec::new();
        if t.len() >= WINDOW {
            for s in 0..=t.len() - WINDOW {
                let mut h = 0xcbf29ce484222325u64;
                let mut distinct = HashSet::new();
                for x in &t[s..s + WINDOW] {
                    h = (h ^ x.0).wrapping_mul(0x100000001b3);
                    distinct.insert(x.0);
                }
                // A run of one kind of token (a table of literals) is not logic.
                let h = if distinct.len() < 14 { 0 } else { h };
                hs.push(h);
                if h != 0 {
                    places.entry(h).or_default().push((fi as u32, s as u32));
                }
            }
        }
        window_hashes.push(hs);
    }
    // Per pair of files: the covered token positions in each.
    let mut covered: HashMap<(u32, u32), (HashSet<u32>, HashSet<u32>)> = HashMap::new();
    for ps in places.values() {
        if ps.len() < 2 || ps.len() > MAX_PLACES {
            continue;
        }
        for (a, &(fa, sa)) in ps.iter().enumerate() {
            for &(fb, sb) in &ps[a + 1..] {
                if fa == fb || files[fa as usize].krate == files[fb as usize].krate {
                    continue;
                }
                let (key, x, y) = if fa < fb {
                    ((fa, fb), sa, sb)
                } else {
                    ((fb, fa), sb, sa)
                };
                let e = covered.entry(key).or_default();
                e.0.extend(x..x + WINDOW as u32);
                e.1.extend(y..y + WINDOW as u32);
            }
        }
    }
    let mut out = BTreeMap::new();
    for ((fa, fb), (x, y)) in covered {
        let n = x.len().min(y.len());
        if n >= MIN_TOKENS {
            out.insert(
                (
                    files[fa as usize].path.clone(),
                    files[fb as usize].path.clone(),
                ),
                n,
            );
        }
    }
    out
}

fn parse_baseline(text: &str) -> BTreeMap<(String, String), usize> {
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .filter_map(|l| {
            let mut w = l.split_whitespace();
            let a = w.next()?;
            let _ = w.next()?;
            let b = w.next()?;
            let n = w.next()?.parse().ok()?;
            Some(((a.to_string(), b.to_string()), n))
        })
        .collect()
}

fn render(m: &BTreeMap<(String, String), usize>) -> String {
    let mut out = String::from(
        "# Duplicate clusters of hand-written Rust across crates (`cargo xtask dupes`): a pair of files and the tokens they share.\n# A ratchet that may only shrink: `cargo xtask dupes --check` fails on a new pair or a larger count;\n# `cargo xtask dupes --baseline` rewrites the counts that fell and drops the pairs that are gone.\n",
    );
    for ((a, b), n) in m {
        out.push_str(&format!("{a} ~ {b} {n}\n"));
    }
    out
}

pub fn run(root: &Path, mode: Mode) -> Result<()> {
    let found = scan(&files(root)?);
    let path = root.join(BASELINE);
    let base = std::fs::read_to_string(&path).ok();
    match mode {
        Mode::Report => {
            let mut v: Vec<_> = found.iter().collect();
            v.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
            for ((a, b), n) in v {
                println!("{n:5} tokens  {a} ~ {b}");
            }
            println!("{} duplicate pairs", found.len());
            Ok(())
        }
        Mode::Baseline => {
            let text = match &base {
                None => render(&found),
                Some(b) => {
                    let old = parse_baseline(b);
                    render(
                        &found
                            .iter()
                            .filter(|(k, _)| old.contains_key(*k))
                            .map(|(k, n)| (k.clone(), (*n).min(old[k])))
                            .collect(),
                    )
                }
            };
            std::fs::write(path, text)?;
            Ok(())
        }
        Mode::Check => {
            let Some(base) = base else {
                bail!("{BASELINE} is missing (run `cargo xtask dupes --baseline`)");
            };
            let old = parse_baseline(&base);
            let mut errors = Vec::new();
            for ((a, b), n) in &found {
                match old.get(&(a.clone(), b.clone())) {
                    None => errors.push(format!(
                        "new duplicate: {a} ~ {b} ({n} tokens): generalize it into a shared module"
                    )),
                    Some(o) if n > o => {
                        errors.push(format!("duplicate grew: {a} ~ {b} ({o} -> {n} tokens)"))
                    }
                    _ => {}
                }
            }
            if errors.is_empty() {
                Ok(())
            } else {
                bail!("dupes:\n  {}", errors.join("\n  "))
            }
        }
    }
}
