use provider_api::format_testing::{
    assert_file_names, assert_formats, assert_lossless, assert_parses, bench_input_files,
    check_corpus,
};

fn provider() -> impl provider_api::LanguageProvider {
    lang_gradle::provider()
}

#[track_caller]
fn fmt(input: &str) -> String {
    assert_formats(provider(), "build.gradle", input)
}

#[test]
fn file_names() {
    assert_file_names(
        &provider(),
        &["build.gradle", "x/settings.gradle"],
        &["build.gradle.kts", "a.groovy"],
    );
}

#[test]
fn bench_inputs_parse() {
    for (name, text) in bench_input_files("gradle") {
        assert_parses(&provider(), &name, &text);
    }
}

/// Commands with several or named arguments after a name or a dotted name parse without errors.
#[test]
fn command_expressions_parse() {
    for text in [
        "dependsOn a, b\n",
        "inputs.property \"k\", v\n",
        "proguardFiles f, 'x'\n",
        "attributes 'k': v\n",
        "foo.bar baz, qux: 1\n",
        "def t = project.tasks.create \"x\", Jar\n",
    ] {
        assert_parses(&provider(), "build.gradle", text);
    }
}

/// Slashy strings, dollar-slashy strings and `\` continuations print back as written.
#[test]
fn slashy_strings_and_continuations_tile() {
    for text in [
        "def r = /a\\/b${x}/\n",
        "def r = $/a/b/$\n",
        "def s = 'a' + \\\n    'b'\n",
    ] {
        assert_lossless(&provider(), "build.gradle", text);
    }
}

#[test]
fn brackets_indent_the_lines() {
    assert_eq!(
        fmt("a {\nb(1,\n2)\nif (x) {\nc\n}\n}\n"),
        "a {\n    b(1,\n        2)\n    if (x) {\n        c\n    }\n}\n"
    );
}

#[test]
fn blanks_around_commas_and_braces() {
    assert_eq!(
        fmt("a { b  1,2 }\nfoo(  1,2 )\nx=[1,2]\ntask t(type:Jar){}\n"),
        "a { b  1, 2 }\nfoo(1, 2)\nx = [1, 2]\ntask t(type: Jar) {}\n"
    );
}

#[test]
fn no_blank_line_after_an_opener_or_before_a_closer() {
    assert_eq!(fmt("a {\n\n  b\n\n}\n"), "a {\n    b\n}\n");
}

/// Every file of the pinned corpus parses, formats with the same tokens and idempotently.
///
/// `cargo test --profile corpus -p lang-gradle --test integration -- --ignored --nocapture conformance`
#[test]
#[ignore]
fn conformance() {
    let (_, unparsed) = check_corpus(provider(), "build.gradle", "gradle", &["gradle"]);
    println!("{unparsed:#?}");
}
