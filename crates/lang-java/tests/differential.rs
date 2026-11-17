//! The hand-written parser against rezel's tree: both build the shapes of
//! `grammars/java.grammar`; the differences that are on purpose (flat compact-source wrappers,
//! flat switch group statements, no empty nodes) are not compared.
//!
//! `JAVA_DUMP=<file>` prints both trees; `differential_corpus` compares them on the corpus.

use core_syntax::{SyntaxElement, SyntaxKind, SyntaxNode, grammars};
use std::collections::BTreeMap;

fn hoisted(kind: SyntaxKind, parent: SyntaxKind) -> bool {
    use lang_java::generated::kind as k;
    kind == k::UNNAMED_COMPILATION_UNIT
        || kind == k::UNIT_BEFORE_FIRST_METHOD
        || (kind == k::SWITCH_GROUP_STATEMENTS && parent == k::SWITCH_GROUP_STATEMENTS)
}

/// The tree without trivia and empty nodes, one element per line, indented by depth.
pub fn dump(node: &SyntaxNode, out: &mut String, depth: usize) {
    let g = grammars::JAVA.get();
    for c in node.children_with_tokens() {
        match c {
            SyntaxElement::Node(n) => {
                if n.text_range().is_empty() {
                    continue;
                }
                if hoisted(n.kind(), node.kind()) {
                    dump(&n, out, depth);
                    continue;
                }
                out.push_str(&format!("{}{}\n", " ".repeat(depth), g.name(n.kind())));
                dump(&n, out, depth + 1);
            }
            SyntaxElement::Token(t) if !g.is_trivia(t.kind()) => {
                out.push_str(&format!(
                    "{}{} {:?}\n",
                    " ".repeat(depth),
                    g.name(t.kind()),
                    t.text()
                ));
            }
            SyntaxElement::Token(_) => {}
        }
    }
}

fn dumped(src: &str, rezel: bool) -> (String, usize) {
    let parsed = if rezel {
        lang_java::parse_rezel(src)
    } else {
        lang_java::parse_handwritten(src)
    };
    assert_eq!(parsed.tree.text().to_string(), src, "round trip");
    let mut out = String::new();
    dump(&parsed.tree, &mut out, 0);
    (out, parsed.errors.len())
}

#[test]
#[ignore]
fn dump_file() {
    let path = std::env::var("JAVA_DUMP").expect("JAVA_DUMP=<file>");
    let src = std::fs::read_to_string(path).unwrap();
    let (a, ea) = dumped(&src, true);
    let (b, eb) = dumped(&src, false);
    std::fs::write("/scratch/slots/sudsy/4/dump-rezel.txt", &a).unwrap();
    std::fs::write("/scratch/slots/sudsy/4/dump-mine.txt", &b).unwrap();
    println!(
        "rezel: {ea} errors; mine: {eb} errors; identical: {}",
        a == b
    );
    for e in lang_java::parse_handwritten(&src).errors.iter().take(8) {
        let line = src[..e.start as usize].matches('\n').count() + 1;
        println!(
            "  mine: line {line}: {} at {:?}",
            e.message,
            &src[e.start as usize..(e.start + e.len.min(30)) as usize]
        );
    }
}

fn java_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            java_files(&p, out);
        } else if p.extension().is_some_and(|e| e == "java") {
            out.push(p);
        }
    }
}

/// Every file of the corpus: parse errors of both parsers, and the first line where the trees differ.
#[test]
#[ignore]
fn differential_corpus() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/conformance/corpus/java");
    let mut files = Vec::new();
    java_files(&root, &mut files);
    let (mut same, mut err_mine, mut err_rezel) = (0, Vec::new(), 0);
    let mut kinds: BTreeMap<String, (usize, String)> = BTreeMap::new();
    for f in &files {
        let Ok(src) = std::fs::read_to_string(f) else {
            continue;
        };
        let (a, ea) = dumped(&src, true);
        let (b, eb) = dumped(&src, false);
        err_rezel += (ea > 0) as usize;
        if eb > 0 {
            err_mine.push(f.display().to_string());
        }
        if a == b {
            same += 1;
            continue;
        }
        let (la, lb): (Vec<_>, Vec<_>) = (a.lines().collect(), b.lines().collect());
        let i = la
            .iter()
            .zip(&lb)
            .position(|(x, y)| x != y)
            .unwrap_or(la.len().min(lb.len()));
        let line = |l: &[&str], i: usize| {
            l.get(i)
                .map_or("<end>".to_string(), |s| s.trim().to_string())
        };
        let key = format!("rezel `{}` / mine `{}`", line(&la, i), line(&lb, i));
        let key: String = key.chars().take(120).collect();
        let e = kinds
            .entry(key)
            .or_insert((0, format!("{}:{}", f.display(), i)));
        e.0 += 1;
    }
    println!(
        "{} files; identical {same}; rezel with errors {err_rezel}; mine with errors {}",
        files.len(),
        err_mine.len()
    );
    for f in err_mine.iter().take(30) {
        println!("  error: {f}");
    }
    let mut v: Vec<_> = kinds.into_iter().collect();
    v.sort_by_key(|(_, (n, _))| std::cmp::Reverse(*n));
    for (key, (n, example)) in v.iter().take(60) {
        println!("{n:5} {key}\n        e.g. {example}");
    }
}
