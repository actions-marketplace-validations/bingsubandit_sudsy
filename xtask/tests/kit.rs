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

fn kit_with(id: &str, shared: bool, reads: &[&str]) -> xtask::kit::Kit {
    let concept = xtask::kit::Concept {
        id: id.to_string(),
        kind: "x".to_string(),
        modules: Vec::new(),
        what: String::new(),
        tools: Vec::new(),
        shared,
        reads: reads.iter().map(|r| r.to_string()).collect(),
        rules: Vec::new(),
        used_by: ["markdown".to_string()].into(),
        local: Vec::new(),
    };
    xtask::kit::Kit {
        modules: Vec::new(),
        concepts: [(id.to_string(), concept)].into(),
        errors: Vec::new(),
        data: Default::default(),
    }
}

#[test]
fn a_shared_concept_that_reads_no_data_may_join_the_single_use_list() {
    let reads = ["option:kind", "rule:a-b", "path:core_format::tabs"];
    let kit = kit_with("a-b", true, &reads);
    assert_eq!(
        xtask::kit::single_use(&kit)["concept a-b"],
        "users: markdown (shared)"
    );
    assert!(xtask::kit::check_ratchet(&kit, "").is_empty());
}

#[test]
fn a_single_use_concept_that_reads_data_or_is_a_language_one_may_not() {
    for kit in [
        kit_with("a-b", true, &["tag:call"]),
        kit_with("a-b", false, &[]),
    ] {
        let errors = xtask::kit::check_ratchet(&kit, "");
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].contains("new entry `concept a-b`"));
    }
}
