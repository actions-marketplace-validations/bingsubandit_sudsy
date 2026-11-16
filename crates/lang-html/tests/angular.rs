use provider_api::format_testing::{
    assert_file_names, assert_formats, assert_lossless, assert_parses, bench_input_files,
    check_corpus,
};

#[track_caller]
fn fmt(input: &str) -> String {
    assert_formats(lang_html::angular_provider(), "a.component.html", input)
}

#[test]
fn file_names() {
    assert_file_names(
        &lang_html::angular_provider(),
        &["app.component.html", "src/a/b.component.html"],
        &["app.html", "component.html", "a.component.htm"],
    );
}

#[test]
fn bench_inputs_parse() {
    for (name, text) in bench_input_files("component.html") {
        assert_parses(&lang_html::angular_provider(), "a.component.html", &text);
        assert!(!name.is_empty());
    }
}

#[test]
fn control_flow_and_let() {
    let input = "@if (a) {<p>x</p>} @else if (b) {y} @else {z}\n@for (i of xs; track i.id) {<li>{{i}}</li>} @empty {none}\n@let n = xs.length;\n";
    let out = fmt(input);
    assert!(out.contains("@if (a) {"), "{out}");
    assert!(out.contains("} @else if (b) {"), "{out}");
    assert!(out.contains("} @empty {"), "{out}");
    assert!(out.contains("@let n = xs.length;"), "{out}");
}

#[test]
fn icu() {
    let out = fmt(
        "<span i18n>{n, plural, =0 {none} =1 {one}}</span>\n<div *ngFor=\"let h of hs;trackBy:tb\" [a]=\"b\" (c)=\"d()\"></div>\n",
    );
    assert!(out.contains("{n, plural, =0 {none} =1 {one}}"), "{out}");
}

#[test]
fn broken_file_is_one_syntax_error() {
    let parsed = assert_lossless(
        &lang_html::angular_provider(),
        "a.component.html",
        "@if (a) {<p>",
    );
    assert_eq!(parsed.errors.len(), 1);
}

/// Real templates, round trip and idempotency: the corpus of `tests/conformance/fetch-angular-corpus.sh`.
///
/// `cargo test --profile corpus -p lang-html --test integration -- --ignored --nocapture angular`
#[test]
#[ignore = "needs the corpus: bash tests/conformance/fetch-angular-corpus.sh"]
fn conformance() {
    for repo in ["components", "angular", "angular-realworld-example-app"] {
        let corpus = format!("angular/{repo}");
        let (n, _) = check_corpus(
            lang_html::angular_provider(),
            "a.component.html",
            &corpus,
            &["html"],
        );
        assert!(n > 0, "{repo}");
    }
}
