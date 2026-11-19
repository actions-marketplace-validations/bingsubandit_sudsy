//! The rules of Ruby: scope rules on the shared binder (`scope`), and the `check` family that the
//! pattern files in `rules/*.yaml` name for rules that read the shape of a call, a class or a
//! clause (RuboCop's Lint department; catalog in `docs/ai/parity/rubocop.yaml`).
//!
//! concept: none (glue: rule registration)

mod calls;
mod classes;
mod clauses;
mod literals;
mod scope;
mod tree;

use crate::generated::kind as k;
use core_rules::checks::{Entry, build};
use core_rules::{Family, Params, Rule, RuleMeta};
use core_syntax::SyntaxKind;

pub static RULES: &[&dyn Rule] = &[
    &scope::UNUSED_LOCAL_VARIABLE,
    &scope::UNUSED_METHOD_ARGUMENT,
    &scope::UNUSED_BLOCK_ARGUMENT,
    &scope::UNDERSCORE_PREFIXED,
];

pub static FAMILIES: &[Family] = &[
    Family {
        name: "check",
        build: build_check,
    },
    core_rules::simplify::FAMILY,
];

const CALL: &[SyntaxKind] = &[k::CALL];
const MODULE_BODY: &[SyntaxKind] = &[k::CLASS, k::MODULE];
const DEFINITION: &[SyntaxKind] = &[k::CONSTANT_WRITE, k::CLASS, k::MODULE];

const CHECKS: &[Entry] = &[
    (
        "ambiguous-block-association",
        CALL,
        calls::ambiguous_block_association,
    ),
    (
        "float-comparison",
        &[k::CALL, k::WHEN],
        literals::float_comparison,
    ),
    ("boolean-symbol", &[k::SYMBOL], literals::boolean_symbol),
    (
        "interpolation-check",
        &[k::STRING],
        literals::interpolation_check,
    ),
    (
        "ineffective-access-modifier",
        MODULE_BODY,
        classes::ineffective_access_modifier,
    ),
    (
        "useless-access-modifier",
        &[k::CLASS, k::MODULE, k::SINGLETON_CLASS, k::CALL, k::PROGRAM],
        classes::useless_access_modifier,
    ),
    (
        "constant-definition-in-block",
        DEFINITION,
        classes::constant_definition_in_block,
    ),
    ("missing-super", &[k::DEF], classes::missing_super),
    ("raise-exception", CALL, calls::raise_exception),
    (
        "parentheses-as-grouped-expression",
        CALL,
        calls::parentheses_as_grouped_expression,
    ),
    (
        "hash-compare-by-identity",
        CALL,
        calls::hash_compare_by_identity,
    ),
    ("struct-new-override", CALL, calls::struct_new_override),
    (
        "redundant-string-coercion",
        &[k::CALL, k::EMBEDDED_STATEMENTS],
        calls::redundant_string_coercion,
    ),
    (
        "literal-in-interpolation",
        &[k::EMBEDDED_STATEMENTS],
        literals::literal_in_interpolation,
    ),
    (
        "mixed-regexp-capture-types",
        &[k::REGULAR_EXPRESSION],
        literals::mixed_regexp_capture_types,
    ),
    ("empty-when", &[k::WHEN], clauses::empty_when),
    (
        "post-condition-loop",
        &[k::WHILE, k::UNTIL],
        clauses::post_condition_loop,
    ),
    (
        "inherit-exception",
        &[k::CLASS, k::CALL],
        classes::inherit_exception,
    ),
    (
        "useless-method-definition",
        &[k::DEF],
        classes::useless_method_definition,
    ),
    (
        "return-in-void-context",
        &[k::RETURN],
        classes::return_in_void_context,
    ),
];

fn build_check(meta: &'static RuleMeta, p: Params) -> Result<Box<dyn Rule>, String> {
    build(CHECKS, meta, p)
}
