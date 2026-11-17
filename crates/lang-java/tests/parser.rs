//! The hand-written parser: deep input, hard literals, and input that is not Java.

use lang_java::parse_handwritten as parse;

fn assert_lossless(src: &str) -> provider_api::ParseResult {
    let parsed = parse(src);
    assert_eq!(parsed.tree.text().to_string(), src);
    parsed
}

#[test]
fn chain_of_10000_calls_parses() {
    let src = format!(
        "class A {{ void f() {{ x{}; }} }}\n",
        ".b(1)".repeat(10_000)
    );
    let parsed = assert_lossless(&src);
    assert!(parsed.errors.is_empty(), "{:?}", parsed.errors.first());
}

#[test]
fn long_operator_and_else_chains_parse() {
    let sum = format!(
        "class A {{ int f() {{ return 1{}; }} }}\n",
        " + 1".repeat(10_000)
    );
    assert!(assert_lossless(&sum).errors.is_empty());
    let ifs = format!(
        "class A {{ void f() {{ if (a) b(); {} }} }}\n",
        "else if (a) b();".repeat(5_000)
    );
    assert!(assert_lossless(&ifs).errors.is_empty());
}

#[test]
fn text_blocks_parse() {
    let src = "class A { String s = \"\"\"\n  a \\\"\"\" b\n  \"\"\"; String t = \"\"\"\n  \"\\\"\"\n  \"\"\"; }\n";
    assert!(assert_lossless(src).errors.is_empty());
}

/// Truncated, bracket-less and garbage input never panics, and the tree keeps every byte.
#[test]
fn broken_input_keeps_every_byte() {
    let sample = "package p;\nimport a.B;\n@A(x = 1) public final class C<T extends D & E> extends F implements G {\n  \
        private int[] f = {1, 2};\n  C(int a) throws X { this(a, 1); }\n  <U> void m(final String... s) {\n    \
        for (int i = 0; i < 3; i++) { x = a ? b : c; }\n    switch (o) { case Foo f when f.x > 1 -> g(); default -> {} }\n    \
        var l = (Runnable) () -> { };\n    String t = \"\"\"\n      hi\n      \"\"\";\n    try (var r = f()) { } catch (A | B e) { } finally { }\n  }\n}\n";
    assert!(assert_lossless(sample).errors.is_empty());
    // Every prefix, and the text with garbage inserted.
    for end in (0..sample.len()).filter(|&i| sample.is_char_boundary(i)) {
        assert_lossless(&sample[..end]);
    }
    for junk in [
        "(", ")", "{", "}", "<", ">", "\"", "'", "/*", "@", "\u{e9}", "\\", "`", "->", "::",
    ] {
        for at in (0..sample.len())
            .step_by(3)
            .filter(|&i| sample.is_char_boundary(i))
        {
            let mut text = sample.to_string();
            text.insert_str(at, junk);
            assert_lossless(&text);
        }
    }
    for src in [
        "",
        "}",
        "{{{{",
        "class",
        "class A {",
        "@",
        "enum E { A(",
        "a.b.",
        "\"\"\"",
        "/* x",
        "'\n",
    ] {
        assert_lossless(src);
    }
}

#[test]
fn nested_brackets_do_not_overflow() {
    let src = format!(
        "class A {{ int x = {}1{}; }}",
        "(".repeat(900),
        ")".repeat(900)
    );
    assert!(assert_lossless(&src).errors.is_empty());
    let src = format!(
        "class A {{ A<{}B{}> x; }}",
        "A<".repeat(500),
        ">".repeat(500)
    );
    assert_lossless(&src);
}

/// Parse speed on the corpus, whole files from memory, best of 3:
/// `cargo test --release -p lang-java --test integration -- --ignored parse_speed --nocapture`.
#[test]
#[ignore]
fn parse_speed() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/conformance/corpus/java");
    let mut texts = Vec::new();
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|e| e == "java")
                && let Ok(t) = std::fs::read_to_string(&p)
            {
                texts.push(t);
            }
        }
    }
    let bytes: usize = texts.iter().map(String::len).sum();
    for (name, f) in [
        (
            "rezel + adapter",
            lang_java::parse_rezel as fn(&str) -> provider_api::ParseResult,
        ),
        ("hand-written", parse),
    ] {
        let best = (0..3)
            .map(|_| {
                let t = std::time::Instant::now();
                for text in &texts {
                    std::hint::black_box(f(text));
                }
                t.elapsed().as_secs_f64()
            })
            .fold(f64::MAX, f64::min);
        println!(
            "{name}: {:.1} MB/s ({} files, {:.1} MB)",
            bytes as f64 / best / 1e6,
            texts.len(),
            bytes as f64 / 1e6
        );
    }
}
