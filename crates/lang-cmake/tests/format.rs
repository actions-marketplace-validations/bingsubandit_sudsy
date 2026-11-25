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
        "set(A\n    \"x   y\n  z\"\n    [=[ a\n b ]=]\n)\n"
    );
    assert_eq!(fmt("set(A -DX=\"a b\" a;b)\n"), "set(A -DX=\"a b\" a;b)\n");
}

#[test]
fn blank_lines_stay_up_to_one_and_none_at_block_edges() {
    assert_eq!(
        fmt("a(x)\n\n\n\nb(y)\nif(A)\n\nc(z)\n\nendif()\n"),
        "a(x)\n\nb(y)\nif(A)\n    c(z)\nendif()\n"
    );
    // An arm keeps the blank lines around it.
    assert_eq!(
        fmt("if(A)\nb(y)\n\nelse()\n\nc(z)\nendif()\n"),
        "if(A)\n    b(y)\n\nelse()\n\n    c(z)\nendif()\n"
    );
}

#[test]
fn gersemi_off_keeps_a_region() {
    assert_eq!(
        fmt("# gersemi: off\nset(  a    b )\n# gersemi: on\nset(  a    b )\n"),
        "# gersemi: off\nset(  a    b )\n# gersemi: on\nset(a b)\n"
    );
}

#[test]
fn a_call_with_more_than_four_values_breaks_and_set_keeps_its_name() {
    assert_eq!(
        fmt(
            "set(Name a b c d e)\nlist(APPEND Names one two three four five)\nlist(APPEND Names one two)\n"
        ),
        "set(Name\n    a\n    b\n    c\n    d\n    e\n)\nlist(\n    APPEND Names\n    one\n    two\n    three\n    four\n    five\n)\nlist(APPEND Names one two)\n"
    );
}

#[test]
fn keyword_sections_nest_and_a_form_without_arguments_opens_a_section() {
    assert_eq!(
        fmt(
            "install(TARGETS foo bar EXPORT x LIBRARY DESTINATION lib COMPONENT dev ARCHIVE DESTINATION lib/static RUNTIME DESTINATION bin)\n"
        ),
        "install(\n    TARGETS foo bar\n    EXPORT x\n    LIBRARY DESTINATION lib COMPONENT dev\n    ARCHIVE DESTINATION lib/static\n    RUNTIME DESTINATION bin\n)\n"
    );
}

#[test]
fn a_command_line_is_filled() {
    assert_eq!(
        fmt(
            "add_custom_command(OUTPUT out.txt COMMAND ${CMAKE_COMMAND} -E copy_if_different ${CMAKE_CURRENT_SOURCE_DIR}/in.txt ${CMAKE_CURRENT_BINARY_DIR}/out.txt DEPENDS in.txt)\n"
        ),
        "add_custom_command(\n    OUTPUT out.txt\n    COMMAND\n        ${CMAKE_COMMAND} -E copy_if_different ${CMAKE_CURRENT_SOURCE_DIR}/in.txt\n        ${CMAKE_CURRENT_BINARY_DIR}/out.txt\n    DEPENDS in.txt\n)\n"
    );
}

#[test]
fn a_condition_breaks_before_its_logical_operators() {
    assert_eq!(
        fmt(
            "if(AAAAAAAAAAAAAAAAAAAAAAAAAAAAAA AND BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB OR CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC)\nendif()\n"
        ),
        "if(\n    AAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\n    AND BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB\n    OR CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC\n)\nendif()\n"
    );
}

#[test]
fn keywords_keep_their_case_and_comments_stay() {
    // A keyword is case-sensitive: `result` is no `RESULT`.
    assert_eq!(
        fmt("find_package(Foo REQUIRED result COMPONENTS a b)\n"),
        "find_package(Foo REQUIRED result COMPONENTS a b)\n"
    );
    assert_eq!(fmt("set(A b # c\n  d)\n"), "set(A\n    b # c\n    d\n)\n");
    assert_eq!(fmt("set(A b) # after\n"), "set(A b) # after\n");
}

#[test]
fn the_signature_table_is_sorted_by_lower_case_name() {
    let names: Vec<String> = lang_cmake::SIGNATURES
        .iter()
        .map(|s| s.name.to_ascii_lowercase())
        .collect();
    assert!(names.windows(2).all(|w| w[0] < w[1]));
    let row = core_syntax::signature::lookup(lang_cmake::SIGNATURES, "EXTERNALPROJECT_ADD");
    assert_eq!(row.map(|s| s.name), Some("ExternalProject_Add"));
}

#[test]
fn a_command_word_with_a_comment_stands_on_a_line_of_its_own() {
    assert_eq!(
        fmt(
            "add_custom_target(t COMMAND ${X} -b a -q # Quiet\n  -t b -D v=1 -D r=2 ${Y} # why\n  \"z\")\n"
        ),
        "add_custom_target(\n    t\n    COMMAND\n        ${X} -b a\n        -q # Quiet\n        -t b -D v=1 -D r=2\n        ${Y} # why\n        \"z\"\n)\n"
    );
}
