use core_syntax::{SyntaxElement, SyntaxNode};
use provider_api::LanguageProvider;
use provider_api::format_testing::{
    assert_file_names, assert_lossless, check_corpus, check_corpus_shapes,
};
use std::path::Path;

fn provider() -> impl LanguageProvider {
    lang_cmake::provider()
}

/// The tree without trivia: nodes as `Name[children]`, tokens as their text.
fn shape(node: &SyntaxNode) -> String {
    let g = core_syntax::grammars::CMAKE.get();
    let parts: Vec<String> = (node.children_with_tokens())
        .filter_map(|e| match e {
            SyntaxElement::Node(n) => Some(shape(&n)),
            SyntaxElement::Token(t) if g.is_trivia(t.kind()) || t.text().is_empty() => None,
            SyntaxElement::Token(t) => Some(t.text().to_string()),
        })
        .collect();
    format!("{}[{}]", g.name(node.kind()), parts.join(" "))
}

/// The shape of the commands of `text` (the `File` node's own brackets left off) and the codes of
/// its errors; the text must print back as written.
#[track_caller]
fn parse(text: &str) -> (String, Vec<String>) {
    let r = assert_lossless(&provider(), "a.cmake", text);
    let s = shape(&r.tree);
    let s = s
        .strip_prefix("File[")
        .and_then(|s| s.strip_suffix(']'))
        .unwrap();
    (
        s.to_string(),
        r.errors.into_iter().map(|e| e.code).collect(),
    )
}

/// The shape of a text that parses without errors.
#[track_caller]
fn ok(text: &str) -> String {
    let (s, errors) = parse(text);
    assert!(errors.is_empty(), "{text:?}: {errors:?}");
    s
}

/// The arguments of the one command `f(<args>)`.
#[track_caller]
fn args(text: &str) -> String {
    let s = ok(&format!("f({text})"));
    let s = s.strip_prefix("Command[f ( ArgumentList[").unwrap();
    s.strip_suffix("] )]").unwrap().to_string()
}

#[test]
fn file_names() {
    assert_file_names(
        &provider(),
        &[
            "CMakeLists.txt",
            "src/CMakeLists.txt",
            "a.cmake",
            "cmake/FindFoo.cmake",
            "arm.toolchain.cmake",
            "vtkEmscriptenCache-6.0.13-wasm64.cmake",
        ],
        &[
            "CMakeLists.txt.in",
            "a.txt",
            "config.h.cmake",
            "foo.pc.cmake",
            "cmake_uninstall.in.cmake",
            "a.cmake.in",
        ],
    );
}

#[test]
fn commands_and_arguments() {
    assert_eq!(ok(""), "");
    assert_eq!(ok("foo()"), "Command[foo ( ArgumentList[] )]");
    assert_eq!(
        ok("set(a b)\nFOO (a)\n"),
        "Command[set ( ArgumentList[Argument[a] Argument[b]] )] Command[FOO ( ArgumentList[Argument[a]] )]"
    );
    // Blocks are not nodes: the closers are commands, paired or not.
    assert_eq!(ok("if(x)else()endif()"), ok("if(x)\nelse()\nendif()"));
}

#[test]
fn unpaired_blocks_are_diagnostics_on_their_command_and_change_no_tree() {
    let text = "set(a)\nendif()\nIf(A)\nelse()\nelseif()\nendforeach()\n";
    let r = lang_cmake::parse(text);
    let found: Vec<_> = (r.errors.iter())
        .map(|e| {
            (
                e.code.as_str(),
                &text[e.start as usize..(e.start + e.len) as usize],
            )
        })
        .collect();
    assert_eq!(
        found,
        [
            ("unpaired-closer", "endif"),
            ("unclosed-block", "If"),
            ("arm-after-last", "elseif"),
            ("unpaired-closer", "endforeach"),
        ]
    );
    // The tree is the flat list of the paired text.
    let (shape, _) = parse(text);
    let (paired, errors) = parse("set(a)\nendif()\nIf(A)\nelse()\nelseif()\nendif()\n");
    assert_eq!(errors, ["unpaired-closer", "arm-after-last"]);
    assert_eq!(shape.matches("Command[").count(), 6);
    assert_eq!(paired.matches("Command[").count(), 6);
}

#[test]
fn paired_blocks_have_no_diagnostic() {
    let text = "function(f)\n  IF(A)\n    foreach(x y)\n    ENDFOREACH()\n  ElseIf(B)\n  else()\n  EndIf()\nendfunction()\n";
    assert!(parse(text).1.is_empty());
}

#[test]
fn bracket_arguments_of_every_level() {
    assert_eq!(args("[[a b]]"), "Argument[[[a b]]]");
    assert_eq!(args("[=[a ]] b]=]"), "Argument[[=[a ]] b]=]]");
    assert_eq!(
        args("[==[a ]=] ]==] x"),
        "Argument[[==[a ]=] ]==]] Argument[x]"
    );
    assert_eq!(
        args("[=====[\n(]====]\n)]=====]"),
        "Argument[[=====[\n(]====]\n)]=====]]"
    );
    // A mismatched closer does not end it; `]]` inside `[=[` is text.
    assert_eq!(args("[=[]]]=]"), "Argument[[=[]]]=]]");
    // Not a bracket: `[` inside a word, `[=` without the second `[`.
    assert_eq!(args("a[[b c]]"), "Argument[a[[b] Argument[c]]]");
    assert_eq!(args("[=x [y"), "Argument[[=x] Argument[[y]");
}

#[test]
fn bracket_comments() {
    assert_eq!(
        ok("#[[ a ]] f(x #[==[ ) ]] ]==] y)"),
        "Command[f ( ArgumentList[Argument[x] Argument[y]] )]"
    );
    assert_eq!(
        ok("#[[a\nb]]x(a)"),
        "Command[x ( ArgumentList[Argument[a]] )]"
    );
    // `#[` and `#[=` are line comments.
    assert_eq!(ok("#[ a\n#[= b\nf()"), "Command[f ( ArgumentList[] )]");
}

#[test]
fn quoted_arguments() {
    assert_eq!(
        args("\"a b\" \"x\\\"y\""),
        "Argument[\"a b\"] Argument[\"x\\\"y\"]"
    );
    // `\` newline continues the string; a bare newline is part of it too.
    assert_eq!(
        args("\"abc\\\ndef\" \"m\nl\" b\\\nc"),
        "Argument[\"abc\\\ndef\"] Argument[\"m\nl\"] Argument[b\\\nc]"
    );
    assert_eq!(args("\"\\\\\" a"), "Argument[\"\\\\\"] Argument[a]");
    assert_eq!(
        args("\")\" \"(\" \"#\""),
        "Argument[\")\"] Argument[\"(\"] Argument[\"#\"]"
    );
    assert_eq!(args("\"\""), "Argument[\"\"]");
}

#[test]
fn semicolons_stay_in_the_argument() {
    assert_eq!(
        args("a;b;c \"x;y\" \\;"),
        "Argument[a;b;c] Argument[\"x;y\"] Argument[\\;]"
    );
}

#[test]
fn glued_words_are_one_argument() {
    assert_eq!(
        args("-DFOO=\"bar\" -DBAZ=\\\"q\\\""),
        "Argument[-DFOO= \"bar\"] Argument[-DBAZ=\\\"q\\\"]"
    );
    assert_eq!(args("a\"b\"c"), "Argument[a \"b\" c]");
    assert_eq!(args("\"a\"b"), "Argument[\"a\" b]");
    assert_eq!(args("\"a\"[[b]]"), "Argument[\"a\" [[b]]]");
    assert_eq!(
        args("$<$<CONFIG:Debug>:dbg;\"x y\">"),
        "Argument[$<$<CONFIG:Debug>:dbg; \"x y\" >]"
    );
    assert_eq!(args("\"a\" b"), "Argument[\"a\"] Argument[b]");
}

#[test]
fn parentheses_are_groups() {
    assert_eq!(
        args("a (b (c)) d"),
        "Argument[a] ParenGroup[( ArgumentList[Argument[b] ParenGroup[( ArgumentList[Argument[c]] )]] )] Argument[d]"
    );
    // Inside a generator expression the parenthesis is a token of its own: `(a b)` splits.
    assert_eq!(
        args("$<$<X:GNU>:(a b)>"),
        "Argument[$<$<X:GNU>:] ParenGroup[( ArgumentList[Argument[a] Argument[b]] )] Argument[>]"
    );
    assert_eq!(args("$<1:a b>"), "Argument[$<1:a] Argument[b>]");
}

#[test]
fn references_and_escapes_are_words() {
    assert_eq!(
        args("${a${b}} $ENV{X} @VAR@ \"${x}\" $CACHE{y}"),
        "Argument[${a${b}}] Argument[$ENV{X}] Argument[@VAR@] Argument[\"${x}\"] Argument[$CACHE{y}]"
    );
    assert_eq!(
        args("a\\ b \\(c\\) \\#d \\n"),
        "Argument[a\\ b] Argument[\\(c\\)] Argument[\\#d] Argument[\\n]"
    );
    // A `#` ends a word.
    assert_eq!(args("a#b\n c"), "Argument[a] Argument[c]");
}

#[test]
fn comments_in_argument_lists() {
    let text = "foo(a # c1\n b # c2\n) # after\n# own line\nbar()\n";
    assert_eq!(
        ok(text),
        "Command[foo ( ArgumentList[Argument[a] Argument[b]] )] Command[bar ( ArgumentList[] )]"
    );
    assert_eq!(
        ok("foo(# ) \n a)"),
        "Command[foo ( ArgumentList[Argument[a]] )]"
    );
}

#[test]
fn non_ascii_text() {
    assert_eq!(
        args("é\u{a0}x \"ü\" \\é ✓"),
        "Argument[é\u{a0}x] Argument[\"ü\"] Argument[\\é] Argument[✓]"
    );
    assert_eq!(
        ok("sét(ü 日本)"),
        "Command[sét ( ArgumentList[Argument[ü] Argument[日本]] )]"
    );
}

#[test]
fn line_breaks_and_bom() {
    let crlf = "set(a b)\r\nfoo(\r\n  a\r\n  b\r\n)\r\n";
    assert_eq!(ok(crlf), ok(&crlf.replace("\r\n", "\n")));
    assert_eq!(
        ok("f(a\\\r\nb \"x\\\r\ny\")"),
        "Command[f ( ArgumentList[Argument[a\\\r\nb] Argument[\"x\\\r\ny\"]] )]"
    );
    assert_eq!(
        ok("a(b)\rc(d)\r"),
        "Command[a ( ArgumentList[Argument[b]] )] Command[c ( ArgumentList[Argument[d]] )]"
    );
    assert_eq!(ok("foo(a)"), "Command[foo ( ArgumentList[Argument[a]] )]");
}

#[test]
fn stray_tokens_are_one_error() {
    let (s, e) = parse("@PACKAGE_INIT@\nset(a b)\n");
    assert_eq!(
        s,
        "ERROR[@PACKAGE_INIT@] Command[set ( ArgumentList[Argument[a] Argument[b]] )]"
    );
    assert_eq!(e, ["expected-command"]);
    let (s, e) = parse("foo(a))\nbar()\n)");
    assert_eq!(
        s,
        "Command[foo ( ArgumentList[Argument[a]] )] ERROR[)] Command[bar ( ArgumentList[] )] ERROR[)]"
    );
    assert_eq!(e, ["expected-command", "expected-command"]);
    let (_, e) = parse("a b c(d)");
    assert_eq!(e, ["expected-command"]);
}

#[test]
fn an_unclosed_parenthesis_ends_at_the_next_command_at_column_0() {
    let (s, e) = parse("foo(a b\nbar(c)\n");
    assert_eq!(
        s,
        "Command[foo ( ArgumentList[Argument[a] Argument[b]]] Command[bar ( ArgumentList[Argument[c]] )]"
    );
    assert_eq!(e, ["unclosed-paren"]);
    // An inner group is cut too; indented commands do not end it.
    let (s, e) = parse("foo(a (b\n  bar(c)\nbaz(d)\n");
    assert_eq!(
        s,
        "Command[foo ( ArgumentList[Argument[a] ParenGroup[( ArgumentList[Argument[b] Argument[bar] ParenGroup[( ArgumentList[Argument[c]] )]]]]] Command[baz ( ArgumentList[Argument[d]] )]"
    );
    assert_eq!(e, ["unclosed-paren", "unclosed-paren"]);
    // No later command: the rest of the file.
    let (s, e) = parse("foo(a\n b");
    assert_eq!(s, "Command[foo ( ArgumentList[Argument[a] Argument[b]]]");
    assert_eq!(e, ["unclosed-paren"]);
    // Paired parentheses are not cut at a command at column 0.
    assert_eq!(
        ok("foo(a\nbar(c)\n)"),
        "Command[foo ( ArgumentList[Argument[a] Argument[bar] ParenGroup[( ArgumentList[Argument[c]] )]] )]"
    );
}

#[test]
fn unterminated_text_ends_before_the_next_command_at_column_0() {
    for (text, code) in [
        ("foo(\"abc\n  bar(x)\nbaz(y)\n", "unterminated-string"),
        ("foo([[abc\nbaz(y)\n", "unterminated-bracket"),
        ("foo(a) #[[abc\nbaz(y)\n", "unterminated-bracket"),
    ] {
        let (s, e) = parse(text);
        assert!(
            s.ends_with("Command[baz ( ArgumentList[Argument[y]] )]"),
            "{text:?}: {s}"
        );
        assert!(e.iter().any(|c| c == code), "{text:?}: {e:?}");
    }
    // No later command: to the end of the file.
    let (s, e) = parse("foo(\"abc\n  bar(x)\n");
    assert_eq!(
        s,
        "Command[foo ( ArgumentList[Argument[\"abc\n  bar(x)\n]]]"
    );
    assert_eq!(e, ["unclosed-paren", "unterminated-string"]);
    let (_, e) = parse("#[==[ a ]=] ]]\n");
    assert_eq!(e, ["unterminated-bracket"]);
}

#[test]
fn deep_nesting_ends_in_an_error_without_panicking() {
    let deep = |n: usize| format!("f({}{})", "(".repeat(n), ")".repeat(n));
    ok(&deep(120));
    let (s, e) = parse(&deep(5000));
    assert_eq!(e[0], "nesting-too-deep");
    assert!(s.contains("ERROR["));
    parse(&"(".repeat(100_000));
    parse(&"f(".repeat(100_000));
}

/// A line deleted from a file damages the commands around it and nothing else.
#[test]
fn a_deleted_line_damages_one_command() {
    let text = "project(p)\nif(A)\n  set(x\n    a\n    \"b c\"\n  )\n  foo(1 2)\nendif()\nbar(\n  x\n)\nbaz(y)\n";
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    for i in 0..lines.len() {
        let cut: String = (lines.iter().enumerate())
            .filter(|&(j, _)| j != i)
            .map(|(_, l)| *l)
            .collect();
        let (s, _) = parse(&cut);
        let last = i == lines.len() - 1;
        assert!(
            last || s.contains("Command[baz ( ArgumentList[Argument[y]] )]"),
            "without line {i}: {s}"
        );
        assert!(s.matches("Command[").count() >= 6, "without line {i}: {s}");
    }
}

/// Files that match the file names and are no CMake: `run_nvcc.cmake` is filled in by
/// `configure_file` (`@VAR@` alone on a line), `clang-format.cmake` holds a YAML file.
const TEMPLATES: [&str; 2] = ["run_nvcc.cmake", "clang-format.cmake"];

/// Every `CMakeLists.txt` and `*.cmake` of the pinned corpus (`tests/conformance/fetch-cmake-corpus.sh`)
/// prints back byte for byte and fits the grammar; the parse errors are all in the templates of
/// other languages (`config.h.cmake`, `.pc.cmake`, [`TEMPLATES`]).
///
/// `cargo test --profile corpus -p lang-cmake --test integration -- --ignored --nocapture conformance`
#[test]
#[ignore]
fn conformance() {
    let (_, unparsed) = check_corpus(provider(), "a.cmake", "cmake", &["cmake", "txt"]);
    let own: Vec<_> = (unparsed.iter())
        .filter(|u| {
            let name = u.split(": ").next().unwrap_or("");
            let claimed = (provider().file_matchers().iter()).any(|m| m.matches(Path::new(name)));
            claimed && !TEMPLATES.iter().any(|t| name.ends_with(t))
        })
        .collect();
    assert!(own.is_empty(), "valid files with syntax errors: {own:#?}");
    check_corpus_shapes(provider(), "cmake", &["cmake", "txt"]);
}

/// The corpus files as UTF-8 text, in the byte order of their paths.
fn corpus() -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance/corpus/cmake");
    let (mut files, mut dirs) = (Vec::new(), vec![root.clone()]);
    while let Some(d) = dirs.pop() {
        for e in std::fs::read_dir(d).unwrap() {
            let path = e.unwrap().path();
            let ext = path.extension().and_then(|e| e.to_str());
            if path.is_dir() {
                dirs.push(path);
            } else if let (Some("cmake" | "txt"), Ok(text)) = (ext, std::fs::read_to_string(&path))
            {
                files.push((
                    path.strip_prefix(&root).unwrap().display().to_string(),
                    text,
                ));
            }
        }
    }
    files.sort();
    files
}

/// Parse speed over the corpus, whole files from memory, one thread, best of 3.
///
/// `cargo test --release -p lang-cmake --test integration -- --ignored --nocapture speed`
#[test]
#[ignore = "a measurement"]
fn speed() {
    let files = corpus();
    let bytes: usize = files.iter().map(|f| f.1.len()).sum();
    let best = (0..3)
        .map(|_| {
            let start = std::time::Instant::now();
            for (_, text) in &files {
                std::hint::black_box(lang_cmake::parse(text));
            }
            start.elapsed()
        })
        .min()
        .unwrap();
    let mb = bytes as f64 / 1e6;
    let ms = best.as_secs_f64() * 1e3;
    println!(
        "{} files, {mb:.1} MB: {ms:.0} ms, {:.1} MB/s",
        files.len(),
        mb / best.as_secs_f64()
    );
}

/// The commands of `text` that are whole (no error inside), as their text from the name to the
/// `)`, and whether the text has no parse error at all.
fn intact_commands(text: &str) -> (Vec<String>, bool) {
    let parsed = lang_cmake::parse(text);
    let g = core_syntax::grammars::CMAKE.get();
    let mut found = Vec::new();
    for n in parsed
        .tree
        .descendants()
        .filter(|n| g.name(n.kind()) == "Command")
    {
        let code: Vec<_> = (n.descendants_with_tokens())
            .filter_map(|e| e.into_token())
            .filter(|t| !g.is_trivia(t.kind()))
            .collect();
        let from = usize::from(code[0].text_range().start());
        let to = usize::from(code[code.len() - 1].text_range().end());
        let error_inside = (parsed.errors.iter())
            .filter(|e| !PAIRING.contains(&e.code.as_str()))
            .any(|e| (from..to).contains(&(e.start as usize)));
        let error_node = n
            .descendants()
            .any(|d| d.kind() == core_syntax::SyntaxKind::ERROR);
        if !error_inside && !error_node {
            found.push(text[from..to].to_string());
        }
    }
    (
        found,
        (parsed.errors.iter()).all(|e| PAIRING.contains(&e.code.as_str())),
    )
}

/// The line-deletion test of the plan: in each of N sampled files (at least 15 lines, no parse
/// error) one random non-blank line is deleted; of the commands that were whole before, how
/// many are not whole (or not there) afterwards. The sampling is the one of the measurement of
/// cmake-fmt and tree-sitter-cmake (xorshift64, seed 0x9E3779B97F4A7C15), over the corpus files
/// in the byte order of their paths.
///
/// `cargo test --release -p lang-cmake --test integration -- --ignored --nocapture recovery`
#[test]
#[ignore = "a measurement"]
fn recovery() {
    let files = corpus();
    let n = 600;
    let mut seed: u64 = 0x9E3779B97F4A7C15;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let (mut trials, mut tries, mut within_2, mut within_5, mut lost_sum) = (0, 0, 0, 0, 0usize);
    while trials < n && tries < n * 50 {
        tries += 1;
        let text = &files[next() as usize % files.len()].1;
        let lines: Vec<&str> = text.split_inclusive('\n').collect();
        if lines.len() < 15 {
            continue;
        }
        let (base, ok) = intact_commands(text);
        if !ok || base.is_empty() {
            continue;
        }
        let candidates: Vec<usize> = (0..lines.len())
            .filter(|&i| !lines[i].trim().is_empty())
            .collect();
        let deleted = candidates[next() as usize % candidates.len()];
        let kept_lines = (lines.iter().enumerate()).filter(|&(i, _)| i != deleted);
        let cut: String = kept_lines.map(|(_, l)| *l).collect();
        let mut pool = std::collections::HashMap::<String, usize>::new();
        for c in intact_commands(&cut).0 {
            *pool.entry(c).or_default() += 1;
        }
        let mut kept = 0;
        for c in &base {
            if let Some(k) = pool.get_mut(c).filter(|k| **k > 0) {
                *k -= 1;
                kept += 1;
            }
        }
        let lost = base.len() - kept;
        trials += 1;
        within_2 += usize::from(lost <= 2);
        within_5 += usize::from(lost <= 5);
        lost_sum += lost;
    }
    println!(
        "trials={trials} lost<=2: {within_2} lost<=5: {within_5} mean lost {:.2}",
        lost_sum as f64 / trials as f64
    );
}

/// The codes of the pairing diagnostics: they say a block is unpaired, no command is damaged.
const PAIRING: [&str; 4] = [
    "unclosed-block",
    "unpaired-closer",
    "arm-after-last",
    "unpaired-arm",
];

/// The commands of `text`: lower-case name and byte range from the name to the `)`.
fn commands(text: &str) -> Vec<(String, usize, usize)> {
    let parsed = lang_cmake::parse(text);
    let g = core_syntax::grammars::CMAKE.get();
    let mut found = Vec::new();
    for n in (parsed.tree.descendants()).filter(|n| g.name(n.kind()) == "Command") {
        let code: Vec<_> = (n.descendants_with_tokens())
            .filter_map(|e| e.into_token())
            .filter(|t| !g.is_trivia(t.kind()))
            .collect();
        let from = usize::from(code[0].text_range().start());
        let to = usize::from(code[code.len() - 1].text_range().end());
        found.push((code[0].text().to_ascii_lowercase(), from, to));
    }
    found
}

/// The nesting depth of each command, and how many pairing diagnostics the parse reports.
fn depths(text: &str) -> (Vec<u32>, usize) {
    let mut pairing = core_syntax::block_pair::Pairing::new(&lang_cmake::BLOCKS);
    let d = (commands(text).iter().enumerate())
        .map(|(i, c)| pairing.word(&c.0, i as u32).depth)
        .collect();
    let n = lang_cmake::parse(text)
        .errors
        .iter()
        .filter(|e| PAIRING.contains(&e.code.as_str()))
        .count();
    (d, n)
}

/// Block pairing over the corpus: how many files pair fully; of 600 trials per kind, one opener
/// (`if`) or one closer (`endif`) of a fully paired file deleted, how many of the other commands
/// get another depth, and how many diagnostics there are.
///
/// `cargo test --release -p lang-cmake --test integration -- --ignored --nocapture pairing`
#[test]
#[ignore = "a measurement"]
fn pairing() {
    let files = corpus();
    let mut paired = 0;
    let mut faulty = Vec::new();
    for (name, text) in &files {
        let (_, n) = depths(text);
        if n == 0 {
            paired += 1;
        } else {
            faulty.push(name.clone());
        }
    }
    println!(
        "files {} paired {paired} with pairing faults {} {:?}",
        files.len(),
        faulty.len(),
        &faulty[..faulty.len().min(8)]
    );
    for word in ["endif", "if"] {
        let mut seed: u64 = 0x9E3779B97F4A7C15;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let (mut trials, mut tries) = (0, 0);
        let (mut damage, mut diags) = (Vec::new(), Vec::new());
        while trials < 600 && tries < 60000 {
            tries += 1;
            let text = &files[next() as usize % files.len()].1;
            let (base, n) = depths(text);
            let cmds = commands(text);
            let targets: Vec<usize> = (0..cmds.len()).filter(|&i| cmds[i].0 == word).collect();
            if n != 0 || targets.is_empty() || !lang_cmake::parse(text).errors.is_empty() {
                continue;
            }
            let t = targets[next() as usize % targets.len()];
            let (_, from, to) = &cmds[t];
            let cut = format!("{}{}", &text[..*from], &text[*to..]);
            let (after, n) = depths(&cut);
            assert_eq!(after.len(), base.len() - 1);
            let wrong = (0..base.len())
                .filter(|&i| i != t)
                .filter(|&i| base[i] != after[i - usize::from(i > t)])
                .count();
            trials += 1;
            damage.push(wrong);
            diags.push(n);
        }
        damage.sort();
        let at = |q: usize| damage[damage.len() * q / 100];
        println!(
            "delete {word}: trials {trials} diagnostics mean {:.2}; commands at another depth: zero {} mean {:.1} median {} p90 {} p99 {} max {}",
            diags.iter().sum::<usize>() as f64 / trials as f64,
            damage.iter().filter(|&&d| d == 0).count(),
            damage.iter().sum::<usize>() as f64 / trials as f64,
            at(50),
            at(90),
            at(99),
            damage[damage.len() - 1]
        );
    }
}

/// A file with a BOM reaches the parser without it: the provider's file decoding strips it.
#[test]
fn a_bom_is_stripped_before_the_lexer() {
    let bytes = b"\xef\xbb\xbfset(a b)\n".to_vec();
    let file = provider_api::SourceFile::from_bytes("a.cmake", 1, bytes).unwrap();
    assert!(file.had_bom);
    let parsed = provider().parse(&file).unwrap();
    assert!(parsed.errors.is_empty());
    assert_eq!(parsed.tree.text().to_string(), "set(a b)\n");
}
