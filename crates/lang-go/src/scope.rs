//! What Go says to the shared scope rules (`core_rules::scope_rules`): `../scope.rows`.
//!
//! concept: scope-tables

use core_rules::{Family, ScopeLang, scope_rules};

// Names declared inside a function take effect after their statement.
static LANG: ScopeLang = ScopeLang::new(include_str!("../scope.rows")).late_declares(&[
    "ShortVarDeclaration",
    "VarSpec",
    "ConstSpec",
    "RangeClause",
    "ReceiveStatement",
]);

pub static FAMILIES: &[Family] = &[
    Family {
        name: "scope",
        build: |meta, p| scope_rules::build(&LANG, meta, p),
    },
    core_rules::doc_comments::FAMILY,
    core_rules::doc_style::FAMILY,
    core_rules::unit_doc::FAMILY,
];
