//! `data/spelling.txt` holds spelling rows only; a row `Spellings` does not know fails here.

#[test]
fn every_row_of_the_spelling_file_is_known() {
    core_rules::markup::Spellings::parse_strict(include_str!("../data/spelling.txt")).unwrap();
}
