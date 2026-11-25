//! What PowerShell says to the shared scope rules (`core_rules::scope_rules`): `../scope.rows`,
//! that names are words written `$name` (or `@name`) and compared without case, and that the
//! variables a string expands are in its text.
//!
//! concept: scope-tables

use core_rules::{Family, ScopeLang, missing_arguments, scope_rules};

static LANG: ScopeLang = ScopeLang::new(include_str!("../scope.rows"))
    .expansions(&["String", "HereString"])
    .sigil_names("$@")
    .case_insensitive()
    // `$a[0] = 1` and `$a, $b = ...` read the name; PSScriptAnalyzer reads a parameter in the blocks of these commands only.
    .element_writes(&["[", ","])
    .argument_blocks(&[
        "foreach-object",
        "%",
        "foreach",
        "where-object",
        "?",
        "where",
    ])
    .expansion_escape("`");

pub static FAMILIES: &[Family] = &[
    Family {
        name: "scope",
        build: |meta, p| scope_rules::build(&LANG, meta, p),
    },
    Family {
        name: "missing-arguments",
        build: |meta, p| missing_arguments::build(crate::commands::COMMANDS, meta, p),
    },
];
