//! The options the `csharp/csharpier-compat` preset turns on, one case each (on through the
//! preset, off through the default); `fmt` checks round-trip, idempotency and meaning.

use crate::common::{fmt, fmt_preset, subject};
use core_config::OptionValue;

const PRESET: &str = "csharp/csharpier-compat";

fn compat(input: &str) -> String {
    fmt_preset(input, PRESET)
}

/// The preset with one option set by hand.
fn compat_with(input: &str, name: &str, value: OptionValue) -> String {
    core_format::testing::assert_formats(&subject(&[(name, value)]).with_preset(PRESET), input)
}

#[test]
fn declarators_after_the_first_stand_on_their_own_lines_except_in_for() {
    let input = "class C { int a = 1, b = 2; void M() { for (int i = 0, j = 1; i < j; i++) { } } }";
    assert_eq!(
        compat(input),
        "class C\n{\n    int a = 1,\n        b = 2;\n\n    void M()\n    {\n        for (int i = 0, j = 1; i < j; i++) { }\n    }\n}\n"
    );
    assert!(fmt(input, &[]).contains("int a = 1, b = 2;"));
}

#[test]
fn stacked_using_statements_stay_flush() {
    let input = "class C { void M() { using (var a = A()) using (var b = B()) { x(); } } }";
    assert!(compat(input).contains("using (var a = A())\n        using (var b = B())\n        {"));
    assert!(fmt(input, &[]).contains("using (var a = A())\n            using (var b = B())"));
}

#[test]
fn a_when_clause_hangs_on_a_line_of_its_own() {
    let input = "class C { void M() { try { } catch (Exception e) when (e.Message.Contains(\"a very long message text here\") && e.InnerException != null && flag) { }\n switch (x) { case int y when y > 1000000000000 && y < 20000000000000 && something_else_long_enough_here: break; }\n var r = x switch { _ when someLongConditionName && anotherVeryLongConditionName && third_condition_here => 1, _ => 2 }; } }";
    let out = compat(input);
    assert!(out.contains(
        "catch (Exception e)\n            when (e.Message.Contains(\"a very long message text here\")\n"
    ));
    assert!(out.contains("case int y\n                when y > "));
    // after a discard pattern it stays on the pattern's line
    assert!(out.contains("_ when someLongConditionName"));
    assert!(!fmt(input, &[]).contains("catch (Exception e)\n"));
}

#[test]
fn the_braces_of_a_nested_initializer_attach_to_the_operator_and_a_comment_stays_put() {
    let input = "class C { void M() { var z = new C { P = { A = 1, B = 2 } }; a = [1,2,3]; } }";
    let out = compat(input);
    assert!(out.contains("var z = new C { P = { A = 1, B = 2 } };"));
    assert!(out.contains("a = [1, 2, 3];"));
    // A comment in front of the value is no reason to attach it (it moved on the second format).
    assert_eq!(
        compat("class C { int[] a =\n // c\n { 1, 2 }; }"),
        "class C\n{\n    int[] a =\n        // c\n        { 1, 2 };\n}\n"
    );
}

#[test]
fn an_is_test_keeps_a_conditional_on_the_operator_line() {
    let input = "class C { void M() { var x = value is null ? aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa : bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb; } }";
    assert!(compat(input).contains("var x = value is null\n            ? aaaa"));
    assert!(
        compat_with(input, "assignment_inline_test_kinds", crate::common::l(&[]))
            .contains("var x =\n")
    );
}

#[test]
fn a_ladder_of_conditionals_in_a_return_or_among_arguments_is_indented() {
    let input = "class C { int M() { Foo(1, aaaaaaaaaaaaaaaaaaaaaaaaaaa ? bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb : cccccccccccccccccccccccccccc ? dddddddddddddddddddddddddd : eeeeeeeeeeeeeeeeeeeeeeeeeee); return aaaaaaaaaaaaaaaaaaaaaaaaaaa ? bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb : cccccccccccccccccccccccccccc ? dddddddddddddddddddddddddd : eeeeeeeeeeeeeeeeeeeeeeeeeee; } }";
    let out = compat(input);
    assert!(out.contains("            aaaaaaaaaaaaaaaaaaaaaaaaaaa ? bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\n                : cccc"));
    assert!(out.contains(
        "return aaaaaaaaaaaaaaaaaaaaaaaaaaa ? bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\n            : cccc"
    ));
}

#[test]
fn comments_in_front_of_a_member_chain_do_not_decide_its_breaks() {
    let input = "class C { void M() {\n Foo(\n // c\n a.B().C().D(), 1); } }";
    assert!(compat(input).contains("// c\n            a.B().C().D(),"));
    let off = compat_with(
        input,
        "member_chain_hold_comments",
        OptionValue::Bool(false),
    );
    assert!(off.contains("// c\n                a.B()\n"));
}

#[test]
fn a_creation_and_its_initializer_break_together_and_crowded_ones_expand() {
    let input = "class C { void M() {\n var x = new Foo(aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, cccccccccccccccccccc) { A = 1 }; var y = new List<int> { 1, 2 }; var z = new Foo { A = new Bar { B = 1 }, C = 2 };\n int[][] w = { { 1, 2 }, { 3 } }; } }";
    let out = compat(input);
    assert!(out.contains("        )\n        {\n            A = 1,\n        };"));
    assert!(out.contains("var y = new List<int> { 1, 2 };"));
    assert!(out.contains("var z = new Foo\n        {\n            A = new Bar { B = 1 },"));
    assert!(out.contains("int[][] w =\n        {\n            { 1, 2 },"));
}

#[test]
fn a_cast_operand_that_does_not_fit_hangs_below_the_cast() {
    let input = "class C { void M() { var c = (VeryLongTypeName)someVeryLongFunctionNameeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee.Property.Other; } }";
    assert!(compat(input).contains("(VeryLongTypeName)\n            someVery"));
}

#[test]
fn accessors_without_a_body_collapse_but_ones_with_a_body_keep_their_braces() {
    let out =
        compat("class C {\n int P\n {\n get;\n set;\n }\n int Q\n {\n get { return 1; }\n }\n}");
    assert!(out.contains("    int P { get; set; }\n"));
    assert!(out.contains("    int Q\n    {\n        get { return 1; }\n    }\n"));
}

#[test]
fn only_a_sole_lambda_without_parenthesized_parameters_hugs_the_call() {
    let out = compat(
        "class C { void M() { Run(() => { x(); }); Run((a, b) => { x(); }); Run(1, () => { x(); }); } }",
    );
    assert!(out.contains("Run(() =>\n        {\n            x();\n        });"));
    assert!(out.contains("Run(\n            (a, b) =>\n"));
    assert!(out.contains("Run(\n            1,\n            () =>\n"));
}

#[test]
fn type_arguments_of_one_tuple_array_or_nullable_type_never_break() {
    let out = compat("class C { void M() { Foo<(int, string)> x; Foo<int[]> y; Foo<int?> z; } }");
    assert!(out.contains("Foo<(int, string)> x;\n        Foo<int[]> y;\n        Foo<int?> z;"));
}

#[test]
fn the_resource_of_a_using_breaks_inside_its_parentheses() {
    let out = compat(
        "class C { void M() { using (var xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx = Aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa(bbbbbbbbbbbbbbbbbbbbbbbbbbbbbb)) { } } }",
    );
    assert!(out.contains("using (\n            var xxxx"));
    assert!(out.contains("\n        ) { }"));
}
