//! Lexing, the structure of the tree, and the shared checks (round trip, trivia attachment, the
//! formatter) on probes, the bench inputs and the corpus (`tests/conformance/fetch-cpp-corpus.sh`).

use core_syntax::{SyntaxKind, print};
use lang_cpp::generated::kind;
use lang_cpp::{Dialect, provider};
use provider_api::format_testing::{
    assert_file_names, assert_parses, bench_input_files, check_corpus, check_corpus_shapes,
};
use provider_api::{LanguageProvider, SourceFile};

const CPP: &str = r##"// Modern C++
module;
#include <vector>
#include "a.h"
#define MAX(a, b) ((a) > (b) ? (a) : (b)) \
    + 0
export module m;
namespace a::b { template <typename T> concept C = requires(T t) { { t + t } -> std::same_as<T>; };
auto f(auto x) -> decltype(x) { return x; } }
class Foo : public Bar {
public:
  Foo() : a_(1), b_{2} {}
  auto operator<=>(const Foo&) const = default;
  template <class... Ts> void g(Ts&&... ts) noexcept(true) { (h(ts), ...); }
 private:
  int a_ = 0x1'000, b_[3] = {1, 2, 3};
};
task<int> co() { co_await g(); co_return R"x(raw ")x"; }
auto l = [&, y = 1]<class T>(T t) mutable { return t + y; };
enum class E : unsigned { A = 1, B };
struct S { int a; } s{1}, *p = &s;
int main() try { if constexpr (true) { return 1; } else return 2; } catch (...) { throw; }
"##;

const C: &str = r##"/* Kernel style */
#ifndef _H
#define _H
#include <linux/types.h>
static int __init foo_init(void)
{
	struct s *p = &(struct s){ .a = 1, .b = { [0] = 2 } };
	if (x)
		return -EINVAL;
	else if (y) {
		goto out;
	}
	switch (a) {
	case 1:
		break;
	default:
		;
	}
out:
	return 0;
}
module_init(foo_init);
static DEFINE_MUTEX(lock);
typedef struct { int a; } t_t;
#endif /* _H */
"##;

#[test]
fn probes_parse_and_print_back() {
    assert_parses(&provider(Dialect::Cpp), "a.cc", CPP);
    assert_parses(&provider(Dialect::C), "a.c", C);
}

#[test]
fn file_names() {
    assert_file_names(
        &provider(Dialect::Cpp),
        &["a.cc", "a.cpp", "a.cxx", "a.hpp", "a.hh", "a.hxx", "a.ipp"],
        &["a.c", "a.h", "a.rs"],
    );
    assert_file_names(&provider(Dialect::C), &["a.c", "a.h"], &["a.cpp", "a.hpp"]);
}

#[test]
fn bench_inputs_parse() {
    for (folder, dialect) in [("c", Dialect::C), ("cpp", Dialect::Cpp)] {
        for (name, text) in bench_input_files(folder) {
            assert_parses(&provider(dialect), &name, &text);
        }
    }
}

/// The text of every `kind` node, blanks collapsed.
fn texts(src: &str, dialect: Dialect, wanted: SyntaxKind) -> Vec<String> {
    let parsed = provider(dialect)
        .parse(&SourceFile::new("a.cc", 1, src))
        .unwrap();
    assert_eq!(print(&parsed.tree), src);
    (parsed.tree.descendants())
        .filter(|n| n.kind() == wanted)
        .map(|n| {
            n.text()
                .to_string()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

#[test]
fn c_has_no_cpp_keywords() {
    let src = "int new = 1; class x;";
    let keywords = |dialect| {
        let parsed = provider(dialect)
            .parse(&SourceFile::new("a", 1, src))
            .unwrap();
        (parsed.tree.descendants_with_tokens())
            .filter_map(|e| e.into_token())
            .filter(|t| t.kind() == kind::KEYWORD)
            .count()
    };
    assert_eq!((keywords(Dialect::C), keywords(Dialect::Cpp)), (1, 3));
}

/// Branches that start and end at the same groups are a `PpGroup` of `PpBranch`es; when a
/// branch leaves a group open (`{` in both) or closes one from outside, only the `Directive`
/// nodes are there.
#[test]
fn conditional_groups() {
    let whole = "#if A\nint a;\n#else\nint b;\n#endif\n";
    assert_eq!(texts(whole, Dialect::Cpp, kind::PP_BRANCH).len(), 2);
    assert_eq!(texts(whole, Dialect::Cpp, kind::PP_GROUP).len(), 1);
    let open = "#if A\nvoid f(int a) {\n#else\nvoid f(long a) {\n#endif\n  g();\n}\n";
    assert!(texts(open, Dialect::Cpp, kind::PP_GROUP).is_empty());
    // The branch that leaves the `{` open has a `BraceGroup`; the closed one a `Block`.
    assert_eq!(texts(open, Dialect::Cpp, kind::BLOCK).len(), 1);
    assert_eq!(texts(open, Dialect::Cpp, kind::BRACE_GROUP).len(), 1);
    let closing = "void f() {\n#if A\n}\n#endif\n";
    assert!(texts(closing, Dialect::Cpp, kind::PP_GROUP).is_empty());
    assert_eq!(texts(closing, Dialect::Cpp, kind::DIRECTIVE).len(), 2);
    let nested = "#if A\n#if B\nx;\n#endif\n#elif C\ny;\n#else\nz;\n#endif\n";
    assert_eq!(texts(nested, Dialect::Cpp, kind::PP_BRANCH).len(), 4);
}

/// The deep layer: statements, declarations and expressions are nodes of their own kind.
#[test]
fn deep_nodes() {
    let src = "int f(int a, char* b) {\n  if (a) return a + b * 2;\n  for (;;) g(x->y[1]);\n  int c = a ? 1 : 2;\n}\nclass A : B { int m; };\nauto l = [&](int x) { return x; };\n";
    let one =
        |wanted, text: &str| assert_eq!(texts(src, Dialect::Cpp, wanted), [text], "{wanted:?}");
    one(
        kind::FUNCTION_DECLARATION,
        "int f(int a, char* b) { if (a) return a + b * 2; for (;;) g(x->y[1]); int c = a ? 1 : 2; }",
    );
    one(kind::IF_STATEMENT, "if (a) return a + b * 2;");
    one(kind::FOR_STATEMENT, "for (;;) g(x->y[1]);");
    one(kind::TERNARY, "a ? 1 : 2");
    one(kind::CLASS_SPECIFIER, "class A : B { int m; };");
    one(kind::BASE_CLAUSE, ": B");
    one(kind::LAMBDA, "[&](int x) { return x; }");
    one(kind::MEMBER_ACCESS, "x->y");
    one(kind::INDEX, "x->y[1]");
    // `*` binds tighter than `+`.
    assert_eq!(texts(src, Dialect::Cpp, kind::BINARY).len(), 2);
}

/// Declarators, constructor initializers and a template's declaration are nodes the shapes read.
#[test]
fn declarators_initializers_and_templates() {
    let src =
        "int a = 1, *b;\nstruct S { S() : x(1), y{2} {} } s1, s2;\ntemplate <class T> T id(T v);\n";
    let one = |wanted, expected: &[&str]| assert_eq!(texts(src, Dialect::Cpp, wanted), expected);
    // each declarator and initializer takes the comma after it
    one(kind::DECLARATOR, &["a = 1,", "*b", "s1,", "s2"]);
    one(kind::INITIALIZER, &["x(1),", "y{2}"]);
    one(kind::CONSTRUCTOR_INITIALIZERS, &[": x(1), y{2}"]);
    one(kind::TEMPLATE_PARAMETERS, &["<class T>"]);
    one(
        kind::TEMPLATE_DECLARATION,
        &["template <class T> T id(T v);"],
    );
}

/// Labels, macro calls and `try` are statements of their own; `.f = v` and `[x = x]` hold no assignment.
#[test]
fn deep_precision() {
    let src = "void f(S s) {\n  try { g(); } catch (E&) { h(); }\nout:\n  FOO(s);\n  S t = { .a = s.a };\n  auto l = [x = x] { return x; };\n  y = y;\n}\n";
    let one = |wanted, text: &str| assert_eq!(texts(src, Dialect::Cpp, wanted), [text]);
    one(kind::TRY_STATEMENT, "try { g(); } catch (E&) { h(); }");
    one(kind::LABEL_STATEMENT, "out:");
    one(kind::MACRO_STATEMENT, "FOO(s);");
    // the declarators and `y = y`, not `.a = s.a` or `x = x` of the capture
    assert_eq!(
        texts(src, Dialect::Cpp, kind::ASSIGNMENT),
        ["t = { .a = s.a }", "l = [x = x] { return x; }", "y = y"]
    );
}

#[test]
fn lexing() {
    let src = "#include <a/b.h>\n#define X(a) a##b \\\n  \"s\"\nint x = 1'000 + 0x1p-3; auto s = u8\"x\" R\"d(\n)d\"_s; char c = '\\'';\n";
    let parsed = assert_parses(&provider(Dialect::Cpp), "a.cc", src);
    let tokens: Vec<String> = (parsed.tree.descendants_with_tokens())
        .filter_map(|e| e.into_token())
        .filter(|t| {
            t.kind() == kind::HEADER_NAME
                || t.kind() == kind::STRING
                || t.kind() == kind::CHAR
                || t.kind() == kind::NUMBER
        })
        .map(|t| t.text().to_string())
        .collect();
    assert_eq!(
        tokens,
        [
            "<a/b.h>",
            "\"s\"",
            "1'000",
            "0x1p-3",
            "u8\"x\"",
            "R\"d(\n)d\"_s",
            "'\\''"
        ]
    );
    let directives = texts(src, Dialect::Cpp, kind::DIRECTIVE);
    assert_eq!(
        directives,
        ["#include <a/b.h>", "#define X(a) a##b \\ \"s\""]
    );
}

#[test]
fn never_an_error_but_an_unterminated_comment() {
    let ok = "@ ` \\ ) } ] ' \" x\n#else\n#endif\n#if\n";
    assert!(
        provider(Dialect::Cpp)
            .parse(&SourceFile::new("a.cc", 1, ok))
            .unwrap()
            .errors
            .is_empty()
    );
    let bad = provider(Dialect::Cpp)
        .parse(&SourceFile::new("a.cc", 1, "int a; /* x"))
        .unwrap();
    assert_eq!(bad.errors.len(), 1);
}

/// Every `.c` / `.h` and C++ file of the pinned corpus: `bash tests/conformance/fetch-cpp-corpus.sh &&
/// cargo test --profile corpus -p lang-cpp --test integration -- --ignored --nocapture conformance`.
#[test]
#[ignore = "needs the corpus: tests/conformance/fetch-cpp-corpus.sh"]
fn conformance_c() {
    let (count, unparsed) = check_corpus(provider(Dialect::C), "a.c", "cpp", &["c", "h"]);
    assert!(unparsed.is_empty(), "{count} files: {unparsed:?}");
}

#[test]
#[ignore = "needs the corpus: tests/conformance/fetch-cpp-corpus.sh"]
fn conformance_cpp() {
    let exts = ["cc", "cpp", "cxx", "hpp", "hh", "hxx", "ipp"];
    let (count, unparsed) = check_corpus(provider(Dialect::Cpp), "a.cc", "cpp", &exts);
    assert!(unparsed.is_empty(), "{count} files: {unparsed:?}");
}

/// Speed and the shape of the structure per project: `cargo test --profile corpus -p lang-cpp
/// --test integration -- --ignored --nocapture structure_stats`.
#[test]
#[ignore = "needs the corpus: tests/conformance/fetch-cpp-corpus.sh"]
fn structure_stats() {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance/corpus/cpp");
    for project in std::fs::read_dir(root).unwrap().map(|e| e.unwrap().path()) {
        let mut dirs = vec![project.clone()];
        let (mut files, mut bytes, mut secs) = (0, 0, 0.0);
        let (mut lines, mut ifs, mut groups, mut errors) = (0, 0, 0, 0);
        while let Some(d) = dirs.pop() {
            for path in std::fs::read_dir(d).unwrap().map(|e| e.unwrap().path()) {
                if path.is_dir() {
                    dirs.push(path);
                    continue;
                }
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                let Ok(text) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let dialect = match ext {
                    "c" | "h" => Dialect::C,
                    "cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx" | "ipp" => Dialect::Cpp,
                    _ => continue,
                };
                let start = std::time::Instant::now();
                let parsed = lang_cpp::parse(&text, dialect);
                secs += start.elapsed().as_secs_f64();
                files += 1;
                bytes += text.len();
                errors += parsed.errors.len();
                for n in parsed.tree.descendants() {
                    match n.kind() {
                        kind::LINE => lines += 1,
                        kind::PP_GROUP => groups += 1,
                        kind::DIRECTIVE => {
                            let mut words = (n.descendants_with_tokens())
                                .filter_map(|e| e.into_token())
                                .skip_while(|t| t.kind() != kind::HASH)
                                .filter(|t| matches!(t.kind(), kind::IDENT | kind::KEYWORD));
                            ifs += usize::from(
                                words.next().is_some_and(|t| t.text().starts_with("if")),
                            );
                        }
                        _ => {}
                    }
                }
            }
        }
        println!(
            "{}: files {files}, {:.1} MB/s, lines {lines}, #if {ifs} of which groups {groups}, errors {errors}",
            project.file_name().unwrap().to_string_lossy(),
            bytes as f64 / 1e6 / secs
        );
    }
}

#[test]
fn a_backslash_before_a_line_break_continues_the_string() {
    let src = "const char* s = \"a\\\r\nb\"; char c = '\\\\'; char* t = \"open\nint x;\n";
    let parsed = assert_parses(&provider(Dialect::Cpp), "a.cc", src);
    let strings: Vec<String> = (parsed.tree.descendants_with_tokens())
        .filter_map(|e| e.into_token())
        .filter(|t| t.kind() == kind::STRING || t.kind() == kind::CHAR)
        .map(|t| t.text().to_string())
        .collect();
    assert_eq!(strings, ["\"a\\\r\nb\"", "'\\\\'", "\"open"]);
}

#[test]
#[ignore]
fn dump_tree() {
    let path = std::env::var("DUMP").unwrap();
    let src = std::fs::read_to_string(&path).unwrap();
    let dialect = if path.ends_with(".c") {
        Dialect::C
    } else {
        Dialect::Cpp
    };
    let parsed = provider(dialect)
        .parse(&SourceFile::new("a.cc", 1, &src))
        .unwrap();
    print!(
        "{}",
        core_syntax::dump_tree(&parsed.tree, core_syntax::grammars::CPP.get())
    );
}

#[test]
#[ignore = "needs the corpus: tests/conformance/fetch-cpp-corpus.sh"]
fn shapes_cpp() {
    let exts = ["cc", "cpp", "cxx", "hpp", "hh", "hxx", "ipp"];
    check_corpus_shapes(provider(Dialect::Cpp), "cpp", &exts);
}

#[test]
#[ignore = "needs the corpus: tests/conformance/fetch-cpp-corpus.sh"]
fn shapes_c() {
    check_corpus_shapes(provider(Dialect::C), "cpp", &["c", "h"]);
}

fn fmt(input: &str) -> String {
    use provider_api::format_testing::Formatting;
    let mut formatting = Formatting::new(provider(Dialect::Cpp), "a.cc");
    formatting.options = vec![("indent_goto_labels", core_config::OptionValue::Bool(false))];
    core_format::testing::assert_formats(&formatting, input)
}

#[test]
fn spacing_follows_the_rows_and_the_source_layout_stays() {
    assert_eq!(
        fmt("int f(int a,int b){for(int i=0;i<3;i++){g(a ,b);}return a+b;}\n"),
        "int f(int a, int b) {\n  for (int i = 0; i < 3; i++) {\n    g(a, b);\n  }\n  return a + b;\n}\n"
    );
    assert_eq!(
        fmt("void f() { q = static_cast<int> (x); p = a?b:c; v.operator== (w); }\n"),
        "void f() {\n  q = static_cast<int>(x);\n  p = a ? b : c;\n  v.operator==(w);\n}\n"
    );
}

#[test]
fn short_bodies_join_the_line_when_they_fit() {
    let src = "struct S {\n  int f() { return 1; }\n  int g() {\n    return 2;\n  }\n  enum E { A, B };\n};\n";
    assert_eq!(
        fmt(src),
        "struct S {\n  int f() { return 1; }\n  int g() { return 2; }\n  enum E { A, B };\n};\n"
    );
}

#[test]
fn access_specifiers_cases_and_labels_are_outdented() {
    assert_eq!(
        fmt(
            "class A {\n  int x;\n\n  // kept\n\npublic:\n  // moves\nprivate:\n  int y;\n};\nvoid f() {\n  switch (x) {\n  case 1: {\n    break;\n  }\n  }\n  again:\n  g();\n}\n"
        ),
        "class A {\n  int x;\n\n  // kept\n\npublic:\n  // moves\nprivate:\n  int y;\n};\nvoid f() {\n  switch (x) {\n  case 1: {\n    break;\n  }\n  }\nagain:\n  g();\n}\n"
    );
}

#[test]
fn wrapped_lines_keep_their_hang_under_a_chain() {
    let src = "void f() {\n  P = U(S)\n          .format(A, false,\n                  0);\n#if X\n  g();\n#endif\n}\n";
    assert_eq!(fmt(src), src);
}

#[test]
fn trailing_comments_directives_and_template_headers() {
    // aligned comments end at a code-free line; a continuation stays under its comment
    assert_eq!(
        fmt("int a; // x\nint bbb;   // y\n           // z\nint c;\n\nint d;  // w\n"),
        "int a;   // x\nint bbb; // y\n         // z\nint c;\n\nint d; // w\n"
    );
    // a directive's blanks collapse and its backslashes stand at the line width
    let define = fmt("#define  A(x)   \\\n  foo(x);   \\\n  bar(x);\n");
    assert!(define.starts_with("#define A(x)"), "{define}");
    assert!(define.lines().next().unwrap().ends_with('\\'), "{define}");
    // a template header joins its declaration when that fits
    assert_eq!(
        fmt("template <class T>\nint f(T t);\n"),
        "template <class T> int f(T t);\n"
    );
    // `/*name=*/` stays against the value
    assert_eq!(
        fmt("void f() { g(/*a=*/1); }\n"),
        "void f() { g(/*a=*/1); }\n"
    );
}

#[test]
fn extern_c_function_body_is_indented_and_a_no_break_space_ends_a_comment() {
    // a linkage specification in front of a function does not make its body a scope
    let src = "extern \"C\" int f(int a) {\n  g();\n  return 1;\n}\n";
    assert_eq!(fmt(src), src);
    // a no-break space is text, not trailing whitespace
    let src = "int a; // x\u{a0}\n";
    assert_eq!(fmt(src), src);
}

/// The string and char tokens of `src` parsed as the file `id` (the dialect follows the path).
fn strings(id: &str, src: &str) -> Vec<String> {
    let parsed = provider(Dialect::C)
        .parse(&SourceFile::new(id, 1, src))
        .unwrap();
    (parsed.tree.descendants_with_tokens())
        .filter_map(|e| e.into_token())
        .filter(|t| t.kind() == kind::STRING || t.kind() == kind::CHAR)
        .map(|t| t.text().to_string())
        .collect()
}

#[test]
fn a_c_file_has_no_literal_suffix_but_a_header_may_be_cpp() {
    let src = "char *s = \"a\"B; char c = 'x'Y; auto t = \"a\"_sv;";
    assert_eq!(strings("a.c", src), ["\"a\"", "'x'", "\"a\""]);
    assert_eq!(strings("a.h", src), ["\"a\"B", "'x'Y", "\"a\"_sv"]);
}

#[test]
fn a_raw_string_is_one_token_in_every_file() {
    let raw = "R\"d(a \" b  ;  { c\n x)d\"";
    let src = format!("const char *r = {raw}; int R = 1; char *q = R\"x\";");
    for id in ["a.c", "a.h", "a.cc"] {
        assert_eq!(strings(id, &src)[0], raw, "{id}");
    }
}

#[test]
fn a_header_keeps_the_text_of_cpp_strings() {
    let src = "class A {\n  auto s = \"a\"_sv;\n  const char *r = R\"(a \" b  ;  { c\n     x)\";\n  int  y;\n};\n";
    for (id, file) in [("a.h", "a.h"), ("a.hpp", "a.hpp")] {
        let out = if id == "a.h" {
            provider_api::format_testing::assert_formats(provider(Dialect::C), file, src)
        } else {
            provider_api::format_testing::assert_formats(provider(Dialect::Cpp), file, src)
        };
        assert!(out.contains("\"a\"_sv;"), "{id}: {out}");
        assert!(out.contains("R\"(a \" b  ;  { c\n     x)\""), "{id}: {out}");
        assert!(out.contains("  int y;"), "{id}: {out}");
    }
}

fn fmt_pointers(side: &str, derive: bool, input: &str) -> String {
    use core_config::OptionValue as V;
    use provider_api::format_testing::Formatting;
    let mut formatting = Formatting::new(provider(Dialect::Cpp), "a.cc");
    formatting.options = vec![
        ("pointer_alignment", V::String(side.into())),
        ("derive_pointer_alignment", V::Bool(derive)),
    ];
    core_format::testing::assert_formats(&formatting, input)
}

const POINTERS: &str = "void f(int* a, const T &b, U&& c, int**d, char *const e, Args &&...args) {\n  int * x = 0, *y;\n  vector<int *> v;\n  auto p = (char*)q;\n  T& r = x;\n}\nT*g(T *a);\n";

#[test]
fn pointers_and_references_follow_pointer_alignment() {
    assert_eq!(
        fmt_pointers("right", false, POINTERS),
        "void f(int *a, const T &b, U &&c, int **d, char *const e, Args &&...args) {\n  int *x = 0, *y;\n  vector<int *> v;\n  auto p = (char *)q;\n  T &r = x;\n}\nT *g(T *a);\n"
    );
    assert_eq!(
        fmt_pointers("left", false, POINTERS),
        "void f(int* a, const T& b, U&& c, int** d, char* const e, Args&&... args) {\n  int* x = 0, *y;\n  vector<int*> v;\n  auto p = (char*)q;\n  T& r = x;\n}\nT* g(T* a);\n"
    );
    assert_eq!(
        fmt_pointers("middle", false, "void f(int* a, const T &b);\n"),
        "void f(int * a, const T & b);\n"
    );
}

#[test]
fn a_multiplication_or_bitwise_and_keeps_its_blanks() {
    let src = "void f() {\n  y = (a & MASK) * b;\n  x = a * b;\n  y = a * b & c;\n  Foo<N * 2> w;\n  Foo<N * M> z;\n  int n = k * 2;\n}\nT operator*(T a);\nT operator&(T a, T b);\n";
    for side in ["right", "left"] {
        assert_eq!(fmt_pointers(side, false, src), src, "{side}");
    }
}

#[test]
fn the_file_gives_the_pointer_alignment_when_asked() {
    let right = "int *a;\nint *b;\nint* c;\n";
    assert_eq!(
        fmt_pointers("left", true, right),
        "int *a;\nint *b;\nint *c;\n"
    );
    assert_eq!(
        fmt_pointers("left", false, right),
        "int* a;\nint* b;\nint* c;\n"
    );
    let tie = "int *a;\nint* b;\n";
    assert_eq!(fmt_pointers("left", true, tie), "int* a;\nint* b;\n");
    assert_eq!(fmt_pointers("right", true, tie), "int *a;\nint *b;\n");
    assert_eq!(
        fmt_pointers("right", true, "int* a;\nint* b;\nint *c;\n"),
        "int* a;\nint* b;\nint* c;\n"
    );
}

#[test]
fn binary_operators_get_a_blank_on_both_sides() {
    use provider_api::format_testing::Formatting;
    let formatting = Formatting::new(provider(Dialect::Cpp), "a.cc");
    let src = "void f() {\n  x = a*b+1;\n  y = a&&b || c&d;\n  T *p = q;\n  z = (a & MASK) * b;\n  n = sizeof(T **) + f((U)&v) + sizeof(char *[5]);\n  m = x + (uint32_t)*A + (T)&b[i];\n  k = (x)*1000;\n}\nvoid (*fp)(const Foo *Self, Node *N);\n";
    let out = core_format::testing::assert_formats(&formatting, src);
    assert_eq!(
        out,
        "void f() {\n  x = a * b + 1;\n  y = a && b || c & d;\n  T *p = q;\n  z = (a & MASK) * b;\n  n = sizeof(T **) + f((U)&v) + sizeof(char *[5]);\n  m = x + (uint32_t)*A + (T)&b[i];\n  k = (x) * 1000;\n}\nvoid (*fp)(const Foo *Self, Node *N);\n"
    );
}

#[test]
fn braced_lists_are_padded_by_option() {
    use core_config::OptionValue as V;
    use provider_api::format_testing::Formatting;
    let src = "S s = {1,2};\nS t = { };\nint y[] = {{1},{ 2 }};\nauto l = f([&] { x += 2; });\n";
    for (padded, want) in [
        (
            false,
            "S s = {1, 2};\nS t = {};\nint y[] = {{1}, {2}};\nauto l = f([&] { x += 2; });\n",
        ),
        (
            true,
            "S s = { 1, 2 };\nS t = {};\nint y[] = { { 1 }, { 2 } };\nauto l = f([&] { x += 2; });\n",
        ),
    ] {
        let mut formatting = Formatting::new(provider(Dialect::Cpp), "a.cc");
        formatting.options = vec![("braced_list_padding", V::Bool(padded))];
        assert_eq!(core_format::testing::assert_formats(&formatting, src), want);
    }
}

#[test]
fn a_call_with_a_block_is_a_function_unless_it_is_a_loop_macro() {
    use core_config::OptionValue as V;
    use provider_api::format_testing::Formatting;
    let src = "void f() {\n  list_for_each(a, b) { x(); }\n  my_loop(a) { x(); }\n}\n";
    let words = |w: &[&str]| V::List(w.iter().map(|s| V::String((*s).into())).collect());
    let run = |wrapping: &[&str], macros: &[&str]| {
        let mut formatting = Formatting::new(provider(Dialect::Cpp), "a.cc");
        formatting.options = vec![
            ("brace_wrapping", words(wrapping)),
            ("for_each_macros", words(macros)),
        ];
        core_format::testing::assert_formats(&formatting, src)
    };
    assert_eq!(
        run(&["function"], &["my_loop"]),
        "void f()\n{\n  list_for_each(a, b)\n  {\n    x();\n  }\n  my_loop(a) {\n    x();\n  }\n}\n"
    );
    let mut formatting = Formatting::new(provider(Dialect::Cpp), "a.cc");
    formatting.options = vec![
        ("brace_wrapping", words(&["function"])),
        ("split_empty_braces", words(&["function"])),
    ];
    assert_eq!(
        core_format::testing::assert_formats(&formatting, "void f() {}\nstruct S {};\nm(x) {}\n"),
        "void f()\n{\n}\nstruct S {};\nm(x)\n{\n}\n"
    );
    assert_eq!(
        run(&[], &[]),
        "void f() {\n  list_for_each(a, b) {\n    x();\n  }\n  my_loop(a) {\n    x();\n  }\n}\n"
    );
}

#[test]
fn a_cast_has_a_blank_after_it_by_option() {
    use core_config::OptionValue as V;
    use provider_api::format_testing::Formatting;
    let src = "void f() {\n  x = (int) a.b;\n  char *p = (char *) q;\n}\n";
    for (spaced, want) in [
        (
            false,
            "void f() {\n  x = (int)a.b;\n  char *p = (char *)q;\n}\n",
        ),
        (
            true,
            "void f() {\n  x = (int) a.b;\n  char *p = (char *) q;\n}\n",
        ),
    ] {
        let mut formatting = Formatting::new(provider(Dialect::Cpp), "a.cc");
        formatting.options = vec![("space_after_cast", V::Bool(spaced))];
        assert_eq!(core_format::testing::assert_formats(&formatting, src), want);
    }
}

#[test]
fn spacing_of_declarators_in_conditions_templates_and_small_forms() {
    use provider_api::format_testing::Formatting;
    let formatting = Formatting::new(provider(Dialect::Cpp), "a.cc");
    let src = "template <typename... >\nvoid g(Ts &&...args);\ntemplate <typename T>\nvoid h(T&& rng, T&b);\nvoid k(void (*cb)(int* a, char&b), int&c);\nvoid f() {\n  x = a ? : b;\n  asm(\"x\" : \"r\" (v));\n  asm volatile(\"\" : \"=r\" (x) : \"r\" (y));\n  do {\n    y++;\n  }while (z);\n  for (auto&x : v) {\n  }\n  for (i = 0; i < n && ok; ++i) {\n  }\n  for (auto it = e; it && *it; ++it) {\n  }\n  try {\n  } catch (E &e) {\n  }\n  f(Size *x);\n}\n";
    assert_eq!(
        core_format::testing::assert_formats(&formatting, src),
        "template <typename...> void g(Ts &&...args);\ntemplate <typename T> void h(T &&rng, T &b);\nvoid k(void (*cb)(int *a, char &b), int &c);\nvoid f() {\n  x = a ?: b;\n  asm(\"x\" : \"r\"(v));\n  asm volatile(\"\" : \"=r\"(x) : \"r\"(y));\n  do {\n    y++;\n  } while (z);\n  for (auto &x : v) {\n  }\n  for (i = 0; i < n && ok; ++i) {\n  }\n  for (auto it = e; it && *it; ++it) {\n  }\n  try {\n  } catch (E &e) {\n  }\n  f(Size * x);\n}\n"
    );
}

#[test]
fn closing_template_brackets_join_by_option_and_comparisons_stay_apart() {
    use core_config::OptionValue as V;
    use provider_api::format_testing::Formatting;
    let src = "std::map<int, std::vector<std::pair<int, int> > > m;\ntemplate <class T = std::vector<int> >\nstruct S {};\nbool b = a < b > > c;\nint x = a > > 1;\n";
    let run = |join: bool| {
        let mut formatting = Formatting::new(provider(Dialect::Cpp), "a.cc");
        formatting.options = vec![("join_closing_angles", V::Bool(join))];
        core_format::testing::assert_formats(&formatting, src)
    };
    assert_eq!(
        run(true),
        "std::map<int, std::vector<std::pair<int, int>>> m;\ntemplate <class T = std::vector<int>> struct S {};\nbool b = a<b> > c;\nint x = a > > 1;\n"
    );
    assert_eq!(
        run(false),
        "std::map<int, std::vector<std::pair<int, int> > > m;\ntemplate <class T = std::vector<int> > struct S {};\nbool b = a<b> > c;\nint x = a > > 1;\n"
    );
}

#[test]
fn a_comment_before_a_function_does_not_keep_its_body_expanded() {
    use provider_api::format_testing::Formatting;
    let formatting = Formatting::new(provider(Dialect::Cpp), "a.cc");
    assert_eq!(
        core_format::testing::assert_formats(
            &formatting,
            "/*\n * c\n */\nint f() {\n  return 1;\n}\n"
        ),
        "/*\n * c\n */\nint f() { return 1; }\n"
    );
}

#[test]
fn a_macro_call_statement_in_a_body_has_its_arguments_read() {
    use provider_api::format_testing::Formatting;
    let formatting = Formatting::new(provider(Dialect::Cpp), "a.cc");
    assert_eq!(
        core_format::testing::assert_formats(
            &formatting,
            "void f() {\n  CHECK(g(x, (BYTE*)d));\n  CHECK_EQ(a<int*>(b), 1);\n}\n"
        ),
        "void f() {\n  CHECK(g(x, (BYTE *)d));\n  CHECK_EQ(a<int *>(b), 1);\n}\n"
    );
}

#[test]
fn a_mark_that_ends_a_type_is_a_declarator_in_any_group() {
    use provider_api::format_testing::Formatting;
    let formatting = Formatting::new(provider(Dialect::Cpp), "a.cc");
    assert_eq!(
        core_format::testing::assert_formats(
            &formatting,
            "void f() {\n  CPP_ret(S*)(requires X<T&>) g(int*, A&);\n  y = a*b;\n  z = f(a *b, c);\n}\n"
        ),
        "void f() {\n  CPP_ret(S *)(requires X<T &>) g(int *, A &);\n  y = a * b;\n  z = f(a * b, c);\n}\n"
    );
}

#[test]
fn an_operator_behind_a_cast_touches_it_and_one_behind_an_operand_does_not() {
    use provider_api::format_testing::Formatting;
    let formatting = Formatting::new(provider(Dialect::Cpp), "a.cc");
    assert_eq!(
        core_format::testing::assert_formats(
            &formatting,
            "void f() {\n  x = (char *) &b;\n  y = (a) * b;\n  z = sizeof(T)*n + h()*k() + (a + b)*c;\n}\n"
        ),
        "void f() {\n  x = (char *)&b;\n  y = (a)*b;\n  z = sizeof(T) * n + h() * k() + (a + b) * c;\n}\n"
    );
}

#[test]
fn a_brace_of_an_unknown_construct_stays_on_its_line_unless_a_construct_wraps_its_brace() {
    use core_config::OptionValue as V;
    use provider_api::format_testing::Formatting;
    let src = "template <class T> CPP_ret(int)(requires X<T> AND Y<T>) f(T a)\n{\n  return 1;\n}\n";
    let run = |wrap: bool| {
        let mut formatting = Formatting::new(provider(Dialect::Cpp), "a.cc");
        if wrap {
            formatting.options = vec![(
                "brace_wrapping",
                V::List(vec![V::String("function".into())]),
            )];
        }
        core_format::testing::assert_formats(&formatting, src)
    };
    assert_eq!(
        run(false),
        "template <class T> CPP_ret(int)(requires X<T> AND Y<T>) f(T a) {\n  return 1;\n}\n"
    );
    assert_eq!(run(true), src);
}

#[test]
fn a_brace_after_a_directive_line_stays_on_its_own_line() {
    use provider_api::format_testing::Formatting;
    let formatting = Formatting::new(provider(Dialect::Cpp), "a.cc");
    let src = "void f()\n#if defined(X)\n{\n  y;\n}\n#else\n{\n  z;\n}\n#endif\n";
    assert_eq!(core_format::testing::assert_formats(&formatting, src), src);
}

#[test]
fn an_empty_block_with_a_comment_is_stable_when_empty_blocks_collapse() {
    use core_config::OptionValue as V;
    use provider_api::format_testing::Formatting;
    let mut formatting = Formatting::new(provider(Dialect::Cpp), "a.cc");
    formatting.options = vec![("collapse_empty_blocks", V::Bool(true))];
    // (formatting twice gives the same text: `assert_formats` checks it)
    core_format::testing::assert_formats(
        &formatting,
        "void f() {\n  if (t) {/*unused*/}\n  if (t) {}\n}\n",
    );
}

#[test]
fn clang_format_off_and_on_keep_a_region_as_written() {
    use provider_api::format_testing::Formatting;
    let formatting = Formatting::new(provider(Dialect::Cpp), "a.cc");
    let src = "int  a;\n// clang-format off\nint   b  =  1;\n// clang-format on\nint   c;\n";
    let want = "int a;\n// clang-format off\nint   b  =  1;\n// clang-format on\nint c;\n";
    assert_eq!(core_format::testing::assert_formats(&formatting, src), want);
}

#[test]
fn blank_lines_before_a_closing_brace_go_except_in_a_namespace_and_before_a_keyword() {
    use provider_api::format_testing::Formatting;
    let formatting = Formatting::new(provider(Dialect::Cpp), "a.cc");
    let src = "void g() {\n  if (a) {\n    b;\n\n  } else {\n    c;\n\n  }\n  try {\n    b;\n\n  } catch (E) {\n    c;\n  }\n}\nnamespace n {\nint a;\n\n}\n";
    let want = "void g() {\n  if (a) {\n    b;\n\n  } else {\n    c;\n  }\n  try {\n    b;\n\n  } catch (E) {\n    c;\n  }\n}\nnamespace n {\nint a;\n\n}\n";
    assert_eq!(core_format::testing::assert_formats(&formatting, src), want);
}

#[test]
fn empty_bodies_have_one_line_or_two_by_their_construct_and_a_semicolon_body_its_own_line() {
    use core_config::OptionValue as V;
    use provider_api::format_testing::Formatting;
    let mut formatting = Formatting::new(provider(Dialect::Cpp), "a.cc");
    formatting.options = vec![
        ("collapse_empty_blocks", V::Bool(true)),
        ("empty_body_own_line", V::Bool(true)),
    ];
    let src = "void g() {\n  for (;;) {}\n  while (x) {}\n  do {} while (x);\n  try {} catch (E) {}\n  { }\n  for (;;);\n  while (x);\n  auto f = [] {};\n  auto l = [x]\n  { return 1; };\n  auto m = [] noexcept\n  { return 1; };\n  std::vector<std::vector<int*>> v;\n  const auto&[k, v] = f();\n  for (auto&& [_, i] : f()) g(i);\n}\nstruct S {};\nT (*fp)(int);\nT(*fq)(int);\nvoid h(std::vector<std::vector<int>>*, int*[3]);\n";
    let want = "void g() {\n  for (;;) {\n  }\n  while (x) {\n  }\n  do {\n  } while (x);\n  try {\n  } catch (E) {\n  }\n  {\n  }\n  for (;;)\n    ;\n  while (x)\n    ;\n  auto f = [] {};\n  auto l = [x] { return 1; };\n  auto m = [] noexcept {\n    return 1;\n  };\n  std::vector<std::vector<int *>> v;\n  const auto &[k, v] = f();\n  for (auto &&[_, i] : f())\n    g(i);\n}\nstruct S {};\nT (*fp)(int);\nT(*fq)(int);\nvoid h(std::vector<std::vector<int>> *, int *[3]);\n";
    assert_eq!(core_format::testing::assert_formats(&formatting, src), want);
}

#[test]
fn a_macro_body_and_a_condition_are_laid_out_as_the_code_they_are() {
    use core_config::OptionValue as V;
    use provider_api::format_testing::Formatting;
    let mut formatting = Formatting::new(provider(Dialect::Cpp), "a.cc");
    formatting.options = vec![
        ("collapse_empty_blocks", V::Bool(true)),
        ("empty_body_own_line", V::Bool(true)),
    ];
    let src = "#define A(x) do { f(x); g(x); } while (0)\n#define B(a) ({ f(a); g(a); })\n#define C(r) {.name = #r, .offset = off(r)}\n#define D(a) case V::a: return x(a);\n#define E(a) FOO_BAR(&a,b)\n#define F 1 // comment\n#define G(I) (((I) & M1)==S1||((I) & M2)==S2)\n#define H(a) do {\\\n} while (0)\n#define W(C) DenseSet<C *, C##Info> x;\n#define T template<typename...>\n#define U bool N,\n#define V 1 + 0 /* a\n b */\n#if defined(A)&&B>=3\n#elif __has_include(<compare>)\n#endif\nvoid f() {\n  FOO_BAR(&a,b)\n  EXPECT_THROW({ a(); b(); }, E);\n}\n";
    let pad = |s: &str| format!("{s:<79}\\\n");
    let want = [
        pad("#define A(x)"),
        pad("  do {"),
        pad("    f(x);"),
        pad("    g(x);"),
        "  } while (0)\n".to_string(),
        pad("#define B(a)"),
        pad("  ({"),
        pad("    f(a);"),
        pad("    g(a);"),
        "  })\n".to_string(),
        "#define C(r) {.name = #r, .offset = off(r)}\n".to_string(),
        pad("#define D(a)"),
        pad("  case V::a:"),
        "    return x(a);\n".to_string(),
        "#define E(a) FOO_BAR(&a, b)\n#define F 1 // comment\n".to_string(),
        "#define G(I) (((I) & M1) == S1 || ((I) & M2) == S2)\n".to_string(),
        pad("#define H(a)"),
        pad("  do {"),
        "  } while (0)\n".to_string(),
        "#define W(C) DenseSet<C *, C##Info> x;\n#define T template <typename...>\n#define U bool N,\n#define V 1 + 0 /* a\n b */\n#if defined(A) && B >= 3\n#elif __has_include(<compare>)\n#endif\n".to_string(),
        "void f() {\n  FOO_BAR(&a, b)\n  EXPECT_THROW({ a(); b(); }, E);\n}\n".to_string(),
    ]
    .concat();
    assert_eq!(core_format::testing::assert_formats(&formatting, src), want);
}

#[test]
fn a_macro_body_that_fits_one_line_stays_on_the_line_of_its_name_up_to_the_width() {
    use provider_api::format_testing::Formatting;
    let formatting = Formatting::new(provider(Dialect::Cpp), "a.cc");
    let fits = "#define FOLLY_PRAGMA_UNROLL_N(N) _Pragma(FOLLY_PRAGMA_DETAIL_STR(GCC unroll(N)))\n";
    assert_eq!(
        core_format::testing::assert_formats(&formatting, fits),
        fits
    );
    // (a comment behind the body takes no room; a condition stays on the line of its `#if`)
    let long = "#define FOLLY_HAS_EXCEPTIONS 1 + 0 // default assumption for unknown platforms\n#if defined(AAAAAAAAAAAAAAAAAAAAAAAA) && defined(BBBBBBBBBBBBBBBBBBBBBBBBBB) && defined(CCCCCCCCCCCCCCC)\n#endif\n";
    assert_eq!(
        core_format::testing::assert_formats(&formatting, long),
        long
    );
    let src = "#define FOLLY_PRAGMA_UNROLL_NN(N) _Pragma(FOLLY_PRAGMA_DETAIL_STR(GCC unroll(N)))\n";
    let want = format!(
        "{:<79}\\\n  _Pragma(FOLLY_PRAGMA_DETAIL_STR(GCC unroll(N)))\n",
        "#define FOLLY_PRAGMA_UNROLL_NN(N)"
    );
    assert_eq!(core_format::testing::assert_formats(&formatting, src), want);
}

#[test]
fn only_the_tree_for_formatting_reads_the_code_of_directives() {
    let p = provider(Dialect::Cpp);
    let mut file = SourceFile::new("a.cpp", 1, "#define F(a) a + 1\n#if A && B\n#endif\n");
    let count = |file: &SourceFile| p.parse(file).unwrap().tree.descendants().count();
    let plain = count(&file);
    file.for_format = true;
    assert!(count(&file) > plain);
}

#[test]
fn macros_in_declarations_short_bodies_and_comments_before_labels() {
    use provider_api::format_testing::Formatting;
    let formatting = Formatting::new(provider(Dialect::Cpp), "a.cc");
    // (attribute macros after the type, a `template(` macro, `NAME(arg)(params)`, an initializer list
    // that ends on its last value, and comments held at the indentation of their neighbours)
    let src = "static void __init f(void) { return; }\nvoid __iomem *g(int a) { return 0; }\n__CRTDECL operator new(int a) { return 0; }\ntemplate(typename T)(requires C<T>) void h(T t) { return; }\nRANGES_FUNC(count)(I first, S last) { return 1; }\nMEM_STATIC U16 MEM_read16(const void *p) { return 1; }\nnoinline void nf() { return; }\ntemplate <typename... Ts> void k(Ts&... args);\nint a[] = {1, 2,\n           3\n};\nclass A {\n  int x;\n  // c\npublic:\n  int y;\n};\nvoid m() {\n  /* c */ for_each_x(a, b) {\n    q();\n  }\n lbl:\n  // note\n  x();\n}\n";
    let want = "static void __init f(void) { return; }\nvoid __iomem *g(int a) { return 0; }\n__CRTDECL operator new(int a) { return 0; }\ntemplate(typename T)(requires C<T>) void h(T t) { return; }\nRANGES_FUNC(count)(I first, S last) { return 1; }\nMEM_STATIC U16 MEM_read16(const void *p) { return 1; }\nnoinline void nf() { return; }\ntemplate <typename... Ts> void k(Ts &...args);\nint a[] = {1, 2,\n           3};\nclass A {\n  int x;\n  // c\npublic:\n  int y;\n};\nvoid m() {\n  /* c */ for_each_x(a, b) {\n    q();\n  }\nlbl:\n  // note\n  x();\n}\n";
    assert_eq!(core_format::testing::assert_formats(&formatting, src), want);
}

#[test]
fn a_closing_brace_of_a_list_stays_apart_from_comments_directives_and_statements() {
    use provider_api::format_testing::Formatting;
    let formatting = Formatting::new(provider(Dialect::Cpp), "a.cc");
    let src = "int a[] = {1, 2, // x\n};\nint b[] = {\n#ifdef X\n    1,\n#endif\n};\nauto f = [] {\n  int c[] = {1};\n  ;\n};\n";
    assert_eq!(core_format::testing::assert_formats(&formatting, src), src);
    // (a `}` after a directive that ends with a value stays on its own line)
    let after_directive = "int a[] = {\n#ifndef X\n  1,\n#endif\n\n#define F ((size_t(*)(unsigned int, \\\n    const void *))a[8])\n\n}; /* end */\n";
    core_format::testing::assert_formats(&formatting, after_directive);
}

fn fmt_opts(options: Vec<(&'static str, core_config::OptionValue)>, input: &str) -> String {
    use provider_api::format_testing::Formatting;
    let mut formatting = Formatting::new(provider(Dialect::Cpp), "a.cc");
    formatting.options = options;
    core_format::testing::assert_formats(&formatting, input)
}

#[test]
fn a_blank_line_goes_before_an_access_specifier_only_after_a_plain_member() {
    use core_config::OptionValue as V;
    // first item: none; after a member: one; after a trailing comment, a directive or before a comment: as written
    let src = "class A {\npublic:\n  int a;\nprotected:\n  int b; // c\nprivate:\n  int c;\n#endif\npublic:\n  int d;\n  // e\nprivate:\n  int f;\n};\n";
    assert_eq!(
        fmt_opts(vec![], src),
        "class A {\npublic:\n  int a;\n\nprotected:\n  int b; // c\nprivate:\n  int c;\n#endif\npublic:\n  int d;\n  // e\nprivate:\n  int f;\n};\n"
    );
    let never = fmt_opts(
        vec![(
            "empty_line_before_access_modifier",
            V::String("never".into()),
        )],
        "class A {\n  int a;\n\nprivate:\n  int c;\n};\n",
    );
    assert_eq!(never, "class A {\n  int a;\nprivate:\n  int c;\n};\n");
    let always = fmt_opts(
        vec![(
            "empty_line_before_access_modifier",
            V::String("always".into()),
        )],
        "class A {\n  int a; // c\nprivate:\n  int c;\n};\n",
    );
    assert_eq!(
        always,
        "class A {\n  int a; // c\n\nprivate:\n  int c;\n};\n"
    );
}

#[test]
fn a_call_head_is_found_past_a_comment_and_an_ellipsis_pointer_sticks_to_its_name() {
    assert_eq!(
        fmt(
            "void f(int a) { g /* c */ (a); }\ntemplate <class... Ts> void h(Ts&... args, Us &&...more);\n"
        ),
        "void f(int a) { g /* c */ (a); }\ntemplate <class... Ts> void h(Ts &...args, Us &&...more);\n"
    );
}

#[test]
fn a_for_each_macro_is_found_behind_a_comment() {
    use core_config::OptionValue as V;
    let words = |w: &[&str]| V::List(w.iter().map(|w| V::String((*w).into())).collect());
    // a loop macro: its brace stays on the line although `function` braces wrap
    assert_eq!(
        fmt_opts(
            vec![
                ("for_each_macros", words(&["EACH"])),
                ("brace_wrapping", words(&["function"]))
            ],
            "void m() {\n  /* c */ EACH(a, b)\n  {\n    q();\n  }\n}\n"
        ),
        "void m()\n{\n  /* c */ EACH(a, b) {\n    q();\n  }\n}\n"
    );
}

#[test]
fn a_template_header_that_breaks_expands_the_short_body_and_one_that_fits_joins_it() {
    // (the declaration fits its line, with the body, only once the header moved up)
    assert_eq!(
        fmt(
            "template <class T> int fn(T first_argument, T second_argument, T third_arg_xxxxx) { return 1; }\ntemplate <class T> int g(T t) { return 1; }\n"
        ),
        "template <class T>\nint fn(T first_argument, T second_argument, T third_arg_xxxxx) {\n  return 1;\n}\ntemplate <class T> int g(T t) { return 1; }\n"
    );
}

#[test]
fn a_dead_preprocessor_branch_stays_as_written() {
    // (`#if 0` is not formatted, its `#else` is; a comment before it stays a comment)
    assert_eq!(
        fmt("void f() {\n  /* c */\n#if  0   /* x */\n  g( a );\n#else\n  h( a );\n#endif\n}\n"),
        "void f() {\n  /* c */\n#if 0 /* x */\n  g( a );\n#else\n  h(a);\n#endif\n}\n"
    );
}

#[test]
fn clang_spellings_of_operators_casts_and_declarations() {
    // `##` has no blanks, `co_await` has none before its call, `>>` closes two template lists
    assert_eq!(fmt("#define C(a, b) a ## b\n"), "#define C(a, b) a##b\n");
    assert_eq!(
        fmt("struct S { auto operator co_await () const; };\n"),
        "struct S {\n  auto operator co_await() const;\n};\n"
    );
    assert_eq!(fmt("A<B<C...>> x;\n"), "A<B<C...>> x;\n");
    // a member operand of `&` is a binary operand, `new T<int>[n]` an array
    assert_eq!(
        fmt("void f() { x = p->flags & MASK; y = new T<int>[n]; }\n"),
        "void f() {\n  x = p->flags & MASK;\n  y = new T<int>[n];\n}\n"
    );
    // a structured binding needs `auto`, `extern \"C\"` prefixes a declaration
    assert_eq!(
        fmt("void f() { auto [a, b] = g(); x[1] = 2; }\n"),
        "void f() {\n  auto [a, b] = g();\n  x[1] = 2;\n}\n"
    );
    assert_eq!(
        fmt("extern \"C\" int  f(void);\n"),
        "extern \"C\" int f(void);\n"
    );
    assert_eq!(
        fmt("void f() { return LLVM_EXTENSION reinterpret_cast<X>(y); }\n"),
        "void f() { return LLVM_EXTENSION reinterpret_cast<X>(y); }\n"
    );
}

#[test]
fn delete_brackets_sign_casts_attribute_parameters_and_comments_after_brackets() {
    assert_eq!(
        fmt("void f() { delete [] b; }\n"),
        "void f() { delete[] b; }\n"
    );
    // `(uintptr_t)-1` is a cast, `(i)+1` a sum
    assert_eq!(
        fmt("void f() { x = (uintptr_t)-1; y = (i)+1; }\n"),
        "void f() {\n  x = (uintptr_t)-1;\n  y = (i) + 1;\n}\n"
    );
    // an attribute starts a parameter, so `*` belongs to its declarator
    assert_eq!(
        fmt("int g(const char * a, [[maybe_unused]] const char * b);\n"),
        "int g(const char *a, [[maybe_unused]] const char *b);\n"
    );
    // a block comment written against `(` stays there at a line end
    assert!(fmt("void f() { if (/*c*/\n a) {} }\n").contains("if (/*c*/\n"));
}
