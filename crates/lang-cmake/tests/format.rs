use provider_api::format_testing::assert_formats;

#[track_caller]
fn fmt(input: &str) -> String {
    assert_formats(lang_cmake::provider(), "a.cmake", input)
}

#[test]
fn known_commands_fit_or_break_one_argument_per_line() {
    assert_eq!(fmt("SET(  A   b )\n"), "set(A b)\n");
    let long = "add_library(foo STATIC aaaaaaaaaaaaaaaaaaaa bbbbbbbbbbbbbbbbbbbbbbb cccccccccccccccccccccc dddddddddddddddddddd)\n";
    assert_eq!(
        fmt(long),
        "add_library(\n    foo\n    STATIC\n    aaaaaaaaaaaaaaaaaaaa\n    bbbbbbbbbbbbbbbbbbbbbbb\n    cccccccccccccccccccccc\n    dddddddddddddddddddd\n)\n"
    );
}

#[test]
fn names_by_case_rows_and_known_spelling() {
    assert_eq!(fmt("Foo_Bar(a)\n"), "foo_bar(a)\n");
    assert_eq!(fmt("externalproject_add(x)\n"), "ExternalProject_Add(x)\n");
}

#[test]
fn blocks_indent_and_comments_before_arms_stay_in_the_body() {
    assert_eq!(
        fmt("if(A)\nset(X 1)\n  # note\nelse ()\nmessage(\"hi\") # t\nendif()\n"),
        "if(A)\n    set(X 1)\n    # note\nelse()\n    message(\"hi\") # t\nendif()\n"
    );
}

#[test]
fn unknown_commands_move_with_their_indent_and_keep_their_lines() {
    assert_eq!(
        fmt("if(A)\nmy_cmd(a\n  b)\nendif()\n"),
        "if(A)\n    my_cmd(a\n      b\n    )\nendif()\n"
    );
    // Blanks between the name and `(` are no part of the arguments.
    assert_eq!(fmt("My_cmd   (  a  b )\n"), "my_cmd(  a  b )\n");
}

#[test]
fn quoted_and_bracket_text_stay_byte_for_byte() {
    assert_eq!(
        fmt("set(A \"x   y\n  z\" [=[ a\n b ]=])\n"),
        "set(\n    A\n    \"x   y\n  z\"\n    [=[ a\n b ]=]\n)\n"
    );
    assert_eq!(fmt("set(A -DX=\"a b\" a;b)\n"), "set(A -DX=\"a b\" a;b)\n");
}

#[test]
fn blank_lines_stay_up_to_one_and_none_at_block_edges() {
    assert_eq!(
        fmt("a(x)\n\n\n\nb(y)\nif(A)\n\nc(z)\n\nendif()\n"),
        "a(x)\n\nb(y)\nif(A)\n    c(z)\nendif()\n"
    );
}

#[test]
fn gersemi_off_keeps_a_region() {
    assert_eq!(
        fmt("# gersemi: off\nset(  a    b )\n# gersemi: on\nset(  a    b )\n"),
        "# gersemi: off\nset(  a    b )\n# gersemi: on\nset(a b)\n"
    );
}
