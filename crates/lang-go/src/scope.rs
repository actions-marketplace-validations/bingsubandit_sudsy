//! What Go says to the shared scope rules (`core_rules::scope_rules`): `../scope.rows`.
//!
//! concept: scope-tables

use core_rules::{Family, ScopeLang, scope_rules};

// Names declared inside a function take effect after their statement.
static LANG: ScopeLang = ScopeLang::new(include_str!("../scope.rows"))
    .late_declares(&[
        "ShortVarDeclaration",
        "VarSpec",
        "ConstSpec",
        "RangeClause",
        "ReceiveStatement",
    ])
    // The parser reads `h[i][j]` as a type instantiation: its operands are tagged as types.
    .deep_names(
        &["TypeInstantiationExpression"],
        &["TypeIdent", "PackageIdent"],
    );

pub const FAMILY: Family = Family {
    name: "scope",
    build: |meta, p| scope_rules::build(&LANG, meta, p),
};
