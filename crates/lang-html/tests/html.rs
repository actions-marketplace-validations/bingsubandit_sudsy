use core_config::OptionValue;
use core_format::NodeFormatter;
use core_syntax::print;
use lang_html::generated::kind as k;
use provider_api::format_testing::{Formatting, assert_file_names, assert_formats, check_corpus};
use provider_api::{LanguageProvider, SourceFile};

fn parse(src: &str) -> provider_api::ParseResult {
    lang_html::provider()
        .parse(&SourceFile::new("a.html", 1, src))
        .unwrap()
}

#[track_caller]
fn fmt(input: &str) -> String {
    assert_formats(lang_html::provider(), "a.html", input)
}

#[test]
fn parses_and_prints_as_written() {
    let page = "<!DOCTYPE html>\n<html lang=en>\n<head><title>A &amp; <b>x</b></title>\n\
<style>a > b { color: red }</style><script>if (a < b) {}</script></head>\n\
<body>\n<!-- c -->\n<p class=\"a\" hidden id='x'>one<br>two &copy; <![CDATA[ a ]] > b ]]></p>\n\
<textarea><b>x</b></textarea><img src=a.png><svg><path d=\"\"/></svg>\n</body>\n</html>\n";
    let r = parse(page);
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    assert_eq!(print(&r.tree), page);
    fmt(page);
}

/// `textarea` and `title` hold text, not markup; CDATA and the doctype body are tokens; text
/// after a void element is not its content.
#[test]
fn rcdata_cdata_and_doctype() {
    let r = parse("<title><b></title><textarea><b>x</b></textarea><![CDATA[<a>]]><!doctype html>");
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    let count = |kind| {
        r.tree
            .descendants_with_tokens()
            .filter(|e| e.kind() == kind)
            .count()
    };
    assert_eq!(count(k::ELEMENT), 2);
    assert_eq!(count(k::RAW_TEXT), 2);
    assert_eq!(count(k::C_DATA), 1);
    assert_eq!(count(k::DOCTYPE_BODY), 1);
    let r = parse("<p>a<br>b<br><![CDATA[c]]></p>");
    let count = |kind| r.tree.descendants().filter(|n| n.kind() == kind).count();
    assert_eq!(count(k::ELEMENT), 3);
    assert_eq!(count(k::CHUNK), 3);
    assert!(r.tree.descendants().all(|n| n.kind() != k::ELEMENT
        || n.children().all(|c| c.kind() != k::CHUNK)
        || n.children().any(|c| c.kind() == k::END_TAG)));
}

#[test]
fn syntax_errors_are_reported() {
    let r = parse("<div <p>");
    assert!(!r.errors.is_empty());
    assert_eq!(print(&r.tree), "<div <p>");
}

#[test]
fn a_component_end_tag_is_not_the_end_of_a_self_closing_slot() {
    let out = assert_formats(
        lang_html::vue_provider(),
        "a.vue",
        "<template>\n<Slot><slot />\n</Slot>\n</template>\n",
    );
    assert!(out.contains("<slot /> </Slot>"), "{out}");
}

#[test]
fn blocks_inlines_and_implied_end_tags() {
    assert_eq!(
        fmt(
            "<!DOCTYPE html><html><head><title>T</title></head><body><div><p>a</p><p>b</p></div><ul><li>x<li>y</ul></body></html>"
        ),
        "<!doctype html>\n<html>\n  <head>\n    <title>T</title>\n  </head>\n  <body>\n    <div>\n      <p>a</p>\n      <p>b</p>\n    </div>\n    <ul>\n      <li>x</li>\n      <li>y</li>\n    </ul>\n  </body>\n</html>\n"
    );
    assert_eq!(
        fmt("<P CLASS=\" a  b \" id=x>t<BR>u <b>v</b></P>\n"),
        "<p class=\"a b\" id=\"x\">t<br />u <b>v</b></p>\n"
    );
}

#[test]
fn text_fills_and_borrowed_markers() {
    let long = "word ".repeat(30);
    let out = fmt(&format!("<p>{long}</p>"));
    assert!(out.lines().all(|l| l.len() <= 80), "{out}");
    assert!(out.starts_with("<p>\n  word word"), "{out}");
    assert_eq!(
        fmt("<span>a</span><span>b</span>\n"),
        "<span>a</span><span>b</span>\n"
    );
    let wide = format!(
        "<div><span>{0}</span><span>{0}</span></div>",
        "x".repeat(40)
    );
    assert_eq!(
        fmt(&wide),
        format!(
            "<div>\n  <span>{0}</span\n  ><span>{0}</span>\n</div>\n",
            "x".repeat(40)
        )
    );
}

#[test]
fn pre_comments_and_ignore() {
    let pre = "<div>\n  <pre>\nabc\n  de</pre>\n</div>\n";
    assert_eq!(fmt(pre), pre);
    assert_eq!(
        fmt("<p>a <!-- c --> b</p>\n"),
        "<p>\n  a\n  <!-- c -->\n  b\n</p>\n"
    );
    assert_eq!(
        fmt("<p><b>x</b><!-- c --></p>\n"),
        "<p><b>x</b><!-- c --></p>\n"
    );
    let ignored = "<div>\n  <!-- prettier-ignore -->\n  <p   a = 'b'>x   y</p>\n</div>\n";
    assert_eq!(fmt(ignored), ignored);
}

#[test]
fn attributes_and_options() {
    let long = format!("<a href=\"{0}\" title='{0}'>x</a>\n", "u".repeat(45));
    let out = fmt(&long);
    assert_eq!(
        out,
        format!(
            "<a\n  href=\"{0}\"\n  title=\"{0}\"\n  >x</a\n>\n",
            "u".repeat(45)
        )
    );
    let mut subject = Formatting::new(lang_html::provider(), "a.html");
    subject.options = vec![
        ("bracket_same_line", OptionValue::Bool(true)),
        ("single_attribute_per_line", OptionValue::Bool(true)),
    ];
    assert_eq!(
        core_format::testing::assert_formats(&subject, "<a href=\"x\" title=\"y\">z</a>\n"),
        "<a\n  href=\"x\"\n  title=\"y\"\n  >z</a\n>\n"
    );
    subject.options = vec![(
        "whitespace_sensitivity",
        OptionValue::String("ignore".into()),
    )];
    assert_eq!(
        core_format::testing::assert_formats(&subject, "<div><span>a</span><span>b</span></div>\n"),
        "<div>\n  <span>a</span>\n  <span>b</span>\n</div>\n"
    );
}

#[test]
fn file_names() {
    assert_file_names(
        &lang_html::provider(),
        &["a.html", "dir/b.htm"],
        &["a.xml", "a.vue"],
    );
}

/// Real-world HTML (`bash tests/conformance/fetch-html-corpus.sh` first): every file parses and
/// formats with the shared checks, or is reported as a syntax error. Prints the counts recorded
/// in docs/ai/formatting/html.md.
///
/// `cargo test --profile corpus -p lang-html --test integration -- --ignored --nocapture conformance`
#[test]
#[ignore = "needs the corpus: bash tests/conformance/fetch-html-corpus.sh"]
fn conformance() {
    for repo in ["prettier", "bootstrap", "learning-area"] {
        println!("{repo}");
        let corpus = format!("html/{repo}");
        let (n, _) = check_corpus(lang_html::provider(), "a.html", &corpus, &["html", "htm"]);
        assert!(n > 0, "{repo}");
    }
}

fn open_subject() -> Formatting {
    let mut subject = Formatting::new(lang_html::provider(), "a.html");
    subject.options = vec![
        ("dialect", OptionValue::String("template-open".into())),
        (
            "whitespace_sensitivity",
            OptionValue::String("ignore".into()),
        ),
    ];
    subject
}

/// The start and end tags of `text` in order (`<div`, `</div`).
fn tags(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find('<') {
        let tag: String = rest[i..]
            .chars()
            .take_while(|&c| c != '>' && c != ' ' && c != '\n')
            .collect();
        out.push(tag);
        rest = &rest[i + 1..];
    }
    out
}

/// `dialect: template-open` writes no end tag the source lacks and drops none of the source:
/// elements opened in the branches of a block, an `<a>` opened and closed in two blocks, an
/// end tag followed by a start tag inside one block, and omitted `</li>` in a file with actions.
#[test]
fn template_open_keeps_the_end_tags_of_the_source() {
    for input in [
        "{{ if .A }}<div class=\"a\">{{ else }}<div class=\"b\">{{ end }}\n<p>text</p>\n</div>\n",
        "{{ if .A }}<a href=\"x\">{{ end }}\ntext\n{{ if .A }}</a>{{ end }}\n",
        "<div>\n{{ if .A }}</div><div>{{ end }}\n<p>x</p>\n</div>\n",
        "<ul>\n<li>a\n<li>b{{ . }}\n</ul>\n",
    ] {
        let out = core_format::testing::assert_formats(&open_subject(), input);
        assert_eq!(tags(&out), tags(input), "{out}");
    }
}

/// The safety check of `template-open` compares the start and end tags: the generic check skips
/// `EndTag`, so a tree with an added or a dropped end tag is refused here.
#[test]
fn template_open_refuses_added_and_dropped_end_tags() {
    let mut options = core_config::ResolvedOptions::default();
    options.set(
        core_config::opt::DIALECT,
        OptionValue::String("template-open".into()),
    );
    let check = |before: &str, after: &str| {
        lang_html::HtmlFormatter.extra_check_with(&parse(before).tree, &parse(after).tree, &options)
    };
    assert!(check("<div><p>{{ . }}</div>", "<div>\n  <p>{{ . }}</div>\n").is_ok());
    assert!(check("<div><div>{{ . }}</div>", "<div><div>{{ . }}</div></div>").is_err());
    assert!(check("<div><p>{{ . }}</p></div>", "<div><p>{{ . }}</div>").is_err());
    // no action: a block cannot end the element elsewhere, so the end tag may be added
    assert!(check("<div><p>a</div>", "<div><p>a</p></div>").is_ok());
}

/// A file without actions has no block that could end an element elsewhere: its omitted end tags
/// are written (`</li>`), except the one of an element that runs to the end of the file.
#[test]
fn template_open_writes_omitted_end_tags_of_a_file_without_actions() {
    let out =
        core_format::testing::assert_formats(&open_subject(), "<ul>\n<li>a\n<li>b\n</ul>\n<p>x\n");
    assert_eq!(out.matches("</li>").count(), 2, "{out}");
    assert!(!out.contains("</p>"), "{out}");
}

/// A `{{ }}` action that spans lines stays as written, with its line break: the markers of
/// adjacent actions do not move to a neighbor, which gave more blank lines on each format.
#[test]
fn a_multi_line_action_stays_as_written() {
    let out = core_format::testing::assert_formats(
        &open_subject(),
        "<p>\n{{ if .A }}{{ .\n}}{{ else\n}}x{{ end }}\n</p>\n",
    );
    assert!(
        out.contains("{{ .\n}}") && out.contains("{{ else\n}}"),
        "{out}"
    );
    // text right before an action does not take its `{{`
    core_format::testing::assert_formats(&open_subject(), "<div>Fuga.{{\n\n\nx\n}} Magni.</div>\n");
}
