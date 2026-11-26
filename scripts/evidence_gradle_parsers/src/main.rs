//! Parser measurements of the Gradle evidence (docs/ai/design/gradle.md). Built once per grammar candidate
//! (`GRAMMAR_SRC=<ts/name/src> cargo build --release`), or with no grammar for the `lex` modes of the hand-written lexer prototype.
//!   parse <list>        per file: bytes, error and missing nodes, first error, whether leaves and blank gaps tile the source
//!   speed <list>        whole files from memory, warm, best of 5, MB/s
//!   mutants <list> <n>  one deleted `)`, `}` or `{` in up to n error-free files: share of the file inside the outermost error nodes
//!   deep <kind> <n>     `foo(` (paren) or `foo {` (brace) nested n deep
//!   probe <file>        snippets separated by lines `//----`: ok or the first error
//!   lex <list>          the hand-written lexer: files with an unknown token, tiling, balance of brackets
//!   lexspeed <list>     the lexer's speed
mod lex;
use std::time::Instant;
use tree_sitter::{Node, Parser, Tree};

include!(concat!(env!("OUT_DIR"), "/symbol.rs"));

fn files(list: &str) -> Vec<(String, Vec<u8>)> {
    std::fs::read_to_string(list).unwrap().lines().filter(|l| !l.is_empty()).map(|p| (p.to_string(), std::fs::read(p).unwrap())).collect()
}

fn count_errors(n: Node, out: &mut (usize, usize, Option<(usize, usize)>), outer: &mut usize) {
    if n.is_error() || n.is_missing() {
        if n.is_error() { out.0 += 1 } else { out.1 += 1 }
        if out.2.is_none() { out.2 = Some((n.start_byte(), n.end_byte())) }
        if n.is_error() { *outer += n.end_byte() - n.start_byte(); return; }
    }
    if !n.has_error() { return; }
    let mut c = n.walk();
    for ch in n.children(&mut c) { count_errors(ch, out, outer); }
}

/// The first stretch between two leaves (or after the last) that holds more than blanks, or a leaf that starts before the previous one ends.
fn first_gap(t: &Tree, src: &[u8]) -> Option<(usize, usize)> {
    let mut pos = 0;
    let mut cur = t.walk();
    loop {
        let n = cur.node();
        if n.child_count() == 0 {
            if n.start_byte() < pos { return Some((n.start_byte(), pos)); }
            if !src[pos..n.start_byte()].iter().all(|b| b.is_ascii_whitespace()) { return Some((pos, n.start_byte())); }
            pos = n.end_byte();
        }
        if cur.goto_first_child() || cur.goto_next_sibling() { continue; }
        loop {
            if !cur.goto_parent() { return if src[pos..].iter().all(|b| b.is_ascii_whitespace()) { None } else { Some((pos, src.len())) }; }
            if cur.goto_next_sibling() { break; }
        }
    }
}

fn tiles(t: &Tree, src: &[u8]) -> bool { first_gap(t, src).is_none() }

fn parser() -> Parser { let mut p = Parser::new(); p.set_language(&lang()).unwrap(); p }

fn line_of(src: &[u8], at: usize) -> usize { src[..at.min(src.len())].iter().filter(|&&b| b == b'\n').count() + 1 }

fn main() {
    let a: Vec<String> = std::env::args().collect();
    match a[1].as_str() {
        "parse" => {
            let mut p = parser();
            for (name, src) in files(&a[2]) {
                let t = p.parse(&src, None).unwrap();
                let mut e = (0, 0, None);
                let mut outer = 0;
                count_errors(t.root_node(), &mut e, &mut outer);
                let (first, snip) = e.2.map(|(s, en)| (line_of(&src, s), String::from_utf8_lossy(&src[s..en.min(s + 40)]).replace(['\n', '\t', '\r'], " "))).unwrap_or((0, String::new()));
                println!("{name}\t{}\t{}\t{}\t{first}\t{snip}\t{}", src.len(), e.0, e.1, tiles(&t, &src));
            }
        }
        "gaps" => {
            let mut p = parser();
            for (name, src) in files(&a[2]) {
                let t = p.parse(&src, None).unwrap();
                if let Some((x, y)) = first_gap(&t, &src) {
                    println!("{name}:{}\t{}\terrors {}", line_of(&src, x), String::from_utf8_lossy(&src[x..y.min(x + 50)]).replace(['\n', '\r', '\t'], " "), t.root_node().has_error());
                }
            }
        }
        "speed" => {
            let fs = files(&a[2]);
            let total: usize = fs.iter().map(|f| f.1.len()).sum();
            let mut p = parser();
            let mut best = f64::MAX;
            for _ in 0..5 {
                let t0 = Instant::now();
                for (_, s) in &fs { std::hint::black_box(p.parse(s, None).unwrap()); }
                best = best.min(t0.elapsed().as_secs_f64());
            }
            println!("{} files {} bytes best {:.1} ms {:.1} MB/s", fs.len(), total, best * 1e3, total as f64 / 1e6 / best);
        }
        "mutants" => {
            let n: usize = a[3].parse().unwrap();
            let mut p = parser();
            let (mut shares, mut none, mut tried) = (vec![], 0, 0);
            for (_, src) in files(&a[2]) {
                if tried >= n { break; }
                let t = p.parse(&src, None).unwrap();
                if t.root_node().has_error() { continue; }
                let sites: Vec<usize> = src.iter().enumerate().filter(|(_, &b)| b == b')' || b == b'}' || b == b'{').map(|(i, _)| i).collect();
                if sites.is_empty() { continue; }
                let at = sites[(src.len() * 31 + sites.len() * 7) % sites.len()];
                let mut m = src.clone();
                m.remove(at);
                let t = p.parse(&m, None).unwrap();
                tried += 1;
                if !t.root_node().has_error() { none += 1; continue; }
                let (mut e, mut outer) = ((0, 0, None), 0);
                count_errors(t.root_node(), &mut e, &mut outer);
                shares.push(outer as f64 / m.len() as f64);
            }
            shares.sort_by(|x, y| x.partial_cmp(y).unwrap());
            let q = |f: f64| shares.get(((shares.len() as f64 * f) as usize).min(shares.len().saturating_sub(1))).copied().unwrap_or(0.0) * 100.0;
            println!("mutants {tried}: no error node {none}; error bytes of the file median {:.1}% p75 {:.1}% p90 {:.1}%", q(0.5), q(0.75), q(0.9));
        }
        "deep" => {
            let n: usize = a[3].parse().unwrap();
            let s = if a[2] == "paren" { format!("{}x{}", "foo(".repeat(n), ")".repeat(n)) } else { format!("{}x{}", "foo {\n".repeat(n), "}\n".repeat(n)) };
            let t0 = Instant::now();
            let t = parser().parse(s.as_bytes(), None).unwrap();
            println!("deep {} {n}: error {} in {:.0} ms", a[2], t.root_node().has_error(), t0.elapsed().as_secs_f64() * 1e3);
        }
        "probe" => {
            let text = std::fs::read_to_string(&a[2]).unwrap();
            let mut p = parser();
            for (i, s) in text.split("\n//----\n").enumerate() {
                let t = p.parse(s.as_bytes(), None).unwrap();
                let mut e = (0, 0, None);
                let mut outer = 0;
                count_errors(t.root_node(), &mut e, &mut outer);
                println!("probe {i}\t{}\t{}", if t.root_node().has_error() { "ERR" } else { "ok" }, s.lines().next().unwrap_or("").chars().take(60).collect::<String>());
            }
        }
        "lex" => lex::run(&files(&a[2]), false),
        "lexspeed" => lex::run(&files(&a[2]), true),
        _ => eprintln!("modes: parse speed mutants deep probe lex lexspeed"),
    }
}
