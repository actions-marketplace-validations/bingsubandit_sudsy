use core_format::spacing::shadowed_rows;
use core_syntax::Grammar;
use xtask::kit::parse_header;

fn java() -> Grammar {
    let text = std::fs::read_to_string(xtask::repo_root().join("grammars/java.grammar")).unwrap();
    Grammar::parse(&text).unwrap()
}

#[test]
fn a_spacing_row_that_an_earlier_row_always_matches_first_is_reported() {
    let rows = "[space]\n\
        kind=Comma / * : no\n\
        kind=Comma&parent=Block / kind=Semi : yes\n\
        * / kind=Semi|Dot : keep\n\
        kind=Dot / kind=Semi : no\n\
        kind=Dot / kind=Comma : no\n";
    let found = shadowed_rows(&java(), rows);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found[0].contains("kind=Comma&parent=Block / kind=Semi"));
    assert!(found[1].contains("kind=Dot / kind=Semi"));
}

#[test]
fn the_rows_of_every_language_are_valid_and_the_kit_is_complete() {
    xtask::rows(&xtask::repo_root()).unwrap();
    xtask::kit::check(&xtask::repo_root()).unwrap();
}

#[test]
fn headers_declare_concepts_and_new_ones() {
    let h = parse_header(
        "#![allow(x)]\n//! Title.\n//!\n//! concept: a-b, new c-d\n//! what: c-d: A thing.\n//! tools: c-d: Prettier x\n//! reads: rules:blank, tag:call\n\nfn f() {}\n",
    );
    assert_eq!(h.ids, ["a-b", "c-d"]);
    assert_eq!(h.new, ["c-d"]);
    assert_eq!(h.per_id[&("c-d".to_string(), true)], "A thing.");
    assert_eq!(h.reads, ["rules:blank", "tag:call"]);
    assert!(parse_header("fn f() {}\n").ids.is_empty());
    assert!(parse_header("//! concept: none (glue: x)\n").none.is_some());
}
