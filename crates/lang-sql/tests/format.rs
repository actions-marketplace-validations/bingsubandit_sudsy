//! Every expectation passes the shared checks (lossless, meaning, comments, idempotency).

use core_format::DirectiveTable;
use core_format::testing::assert_formats;
use provider_api::format::format_file;
use provider_api::format_testing::Formatting;
use provider_api::{LanguageProvider, SourceFile};

#[test]
fn lays_out_lists_and_keeps_comments() {
    let input = "-- header\nSELECT /* inline */ a,b -- trailing\nFROM t;\n/* block\n   comment */\nselect 1;\n";
    let subject = Formatting::new(lang_sql::provider(), "a.sql");
    let want = "-- header\nSELECT /* inline */\n    a,\n    b -- trailing\nFROM t;\n/* block\n   comment */\nselect 1;\n";
    assert_eq!(assert_formats(&subject, input), want);
}

#[test]
fn errors_are_refused() {
    let provider = lang_sql::provider();
    let file = SourceFile::new("a.sql", 1, "SELEC FROM WHERE ;;");
    let options = provider.option_schema().defaults();
    assert!(format_file(&provider, &file, &options, &DirectiveTable::default()).is_err());
}
