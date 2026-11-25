//! The symbol query: what PHP and Protobuf files declare, per construct, and that an edit inside
//! a body leaves the symbols equal and invalidates nothing that reads them.

use core_config::EditorConfig;
use core_incremental::{Database, Db, File, symbols};
use core_semantics::symbols::{FileSymbols, Flags, MemberKind, Name, Relation, TypeKind};
use provider_api::LanguageProvider;
use salsa::{EventKind, Setter};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// A reader of the symbols, standing for the project index.
#[salsa::tracked(returns(ref))]
fn type_count(db: &dyn Db, file: File) -> usize {
    symbols(db, file)
        .as_ref()
        .ok()
        .and_then(|s| s.as_ref())
        .map_or(0, |s| s.types.len())
}

struct Harness {
    db: Database,
    log: Arc<Mutex<Vec<String>>>,
}

impl Harness {
    fn new() -> Self {
        let log = Arc::new(Mutex::new(Vec::new()));
        let sink = log.clone();
        let providers: Vec<Box<dyn LanguageProvider>> = vec![
            Box::new(lang_php::provider()),
            Box::new(lang_protobuf::provider()),
            Box::new(lang_yaml::provider()),
        ];
        let db = Database::with_event_hook(
            providers,
            Box::new(move |event| {
                if let EventKind::WillExecute { database_key } = event.kind {
                    sink.lock().unwrap().push(format!("{database_key:?}"));
                }
            }),
        );
        Harness { db, log }
    }

    fn file(&self, path: &str, text: &str) -> File {
        File::new(
            &self.db,
            PathBuf::from(path),
            1,
            text.to_string(),
            false,
            EditorConfig::default(),
        )
    }

    fn symbols(&self, path: &str, text: &str) -> FileSymbols {
        let file = self.file(path, text);
        symbols(&self.db, file)
            .as_ref()
            .unwrap()
            .clone()
            .expect("the file has symbols")
    }

    fn runs(&self, query: &str) -> usize {
        self.log
            .lock()
            .unwrap()
            .iter()
            .filter(|k| k.starts_with(query))
            .count()
    }
}

fn php(text: &str) -> FileSymbols {
    Harness::new().symbols("a.php", &format!("<?php\n{text}"))
}

fn proto(text: &str) -> FileSymbols {
    Harness::new().symbols("a.proto", text)
}

fn names(n: &Name) -> Vec<&str> {
    n.iter().map(|s| &**s).collect()
}

fn members(t: &core_semantics::symbols::TypeSymbol) -> Vec<(&str, MemberKind)> {
    t.members.iter().map(|m| (&*m.name, m.kind)).collect()
}

#[test]
fn php_types_have_qualified_names_kinds_and_modifiers() {
    let s = php(
        "namespace App\\Models;\nabstract class A {}\nfinal class B {}\ninterface I {}\ntrait T {}\nenum E: string {}\n",
    );
    let got: Vec<_> = s
        .types
        .iter()
        .map(|t| (&*t.name, t.kind, t.flags))
        .collect();
    assert_eq!(
        got,
        [
            ("App\\Models\\A", TypeKind::Class, Flags::ABSTRACT),
            ("App\\Models\\B", TypeKind::Class, Flags::FINAL),
            ("App\\Models\\I", TypeKind::Interface, 0),
            ("App\\Models\\T", TypeKind::Trait, 0),
            ("App\\Models\\E", TypeKind::Enum, 0),
        ]
    );
    assert!(s.types.iter().all(|t| !t.conditional && !t.incomplete));
}

#[test]
fn php_parents_resolve_through_namespace_and_imports() {
    let s = php(
        "namespace App;\nuse Lib\\{Base, Util as U};\nuse Other\\Countable;\nclass C extends Base implements Countable, \\Iterator, U\\Sub, Local {}\n",
    );
    let parents: Vec<_> = s.types[0]
        .parents
        .iter()
        .map(|p| (p.relation, names(&p.name).join("|")))
        .collect();
    assert_eq!(
        parents,
        [
            (Relation::Extends, "Lib\\Base".to_string()),
            (Relation::Implements, "Other\\Countable".to_string()),
            (Relation::Implements, "Iterator".to_string()),
            (Relation::Implements, "Lib\\Util\\Sub".to_string()),
            (Relation::Implements, "App\\Local".to_string()),
        ]
    );
}

#[test]
fn php_members_of_every_kind_with_parameter_counts() {
    let s = php(
        "class C {\n use T1, T2;\n const A = 1, B = 2;\n public static int $x = 0, $y;\n private ?string $name;\n \
         public function __construct(private int $id, public readonly string $n = 'x', ...$rest) {}\n \
         public static function make(int $a, $b = 1): static {}\n \
         abstract protected function foo(): void;\n}\n",
    );
    let c = &s.types[0];
    assert_eq!(
        members(c),
        [
            ("A", MemberKind::Const),
            ("B", MemberKind::Const),
            ("x", MemberKind::Property),
            ("y", MemberKind::Property),
            ("name", MemberKind::Property),
            ("id", MemberKind::Property),
            ("n", MemberKind::Property),
            ("__construct", MemberKind::Method),
            ("make", MemberKind::Method),
            ("foo", MemberKind::Method),
        ]
    );
    assert_eq!(c.mixins.len(), 2);
    assert_eq!(names(&c.mixins[0]), ["T1"]);
    let p = |name: &str| c.members.iter().find(|m| &*m.name == name).unwrap();
    assert_eq!(p("x").flags, Flags::STATIC);
    assert_eq!(p("name").flags, Flags::PRIVATE);
    assert_eq!(p("n").flags, Flags::READONLY);
    let params = |name: &str| {
        let q = p(name).params.unwrap();
        (q.min, q.max, q.variadic)
    };
    assert_eq!(params("__construct"), (1, 2, true));
    assert_eq!(params("make"), (1, 2, false));
    assert_eq!(params("foo"), (0, 0, false));
    assert_eq!(p("make").flags, Flags::STATIC);
    assert_eq!(p("foo").flags, Flags::ABSTRACT | Flags::PROTECTED);
    assert!(!c.incomplete && !c.dynamic && !c.adapted);
}

#[test]
fn php_enum_cases_and_interface_methods() {
    let s = php(
        "enum E: string implements I { case A = 'a'; case B; const X = 1; public function f() {} }\ninterface I { public function f(int $a); }\n",
    );
    assert_eq!(
        members(&s.types[0]),
        [
            ("A", MemberKind::Case),
            ("B", MemberKind::Case),
            ("X", MemberKind::Const),
            ("f", MemberKind::Method),
        ]
    );
    let f = &s.types[1].members[0];
    assert_ne!(f.flags & Flags::ABSTRACT, 0);
    assert_eq!(f.params.unwrap().min, 1);
}

#[test]
fn php_functions_and_constants_take_the_namespace() {
    let s = php("namespace A\\B;\nfunction top(int $a, int ...$b) {}\nconst TOP = 1, TWO = 2;\n");
    assert_eq!(s.functions.len(), 1);
    assert_eq!(&*s.functions[0].name, "A\\B\\top");
    let p = s.functions[0].params.unwrap();
    assert_eq!((p.min, p.max, p.variadic), (1, 1, true));
    let c: Vec<_> = s.constants.iter().map(|c| &*c.name).collect();
    assert_eq!(c, ["A\\B\\TOP", "A\\B\\TWO"]);
}

#[test]
fn php_braced_namespaces_scope_their_imports() {
    let s = php(
        "namespace A { use X\\Y; class P extends Y {} }\nnamespace B { class Q extends Y {} }\nnamespace { class R {} }\n",
    );
    let parent = |i: usize| names(&s.types[i].parents[0].name)[0].to_string();
    assert_eq!(&*s.types[0].name, "A\\P");
    assert_eq!(parent(0), "X\\Y");
    assert_eq!(&*s.types[1].name, "B\\Q");
    assert_eq!(parent(1), "B\\Y");
    assert_eq!(&*s.types[2].name, "R");
}

#[test]
fn php_use_function_and_const_import_their_own_kind() {
    let s = php(
        "namespace A;\nuse function X\\helper as h;\nuse const X\\VERSION;\nfunction f() { return 1; }\nclass C extends h {}\n",
    );
    // `h` is a function alias: a class name `h` is not it.
    assert_eq!(names(&s.types[0].parents[0].name), ["A\\h"]);
}

#[test]
fn php_declarations_in_branches_and_functions_are_conditional() {
    let s = php(
        "if (!class_exists('X')) { class X {} }\nfunction f() { class Inner {} function g() {} }\nclass Top {}\ntry { class T {} } catch (\\Exception $e) {}\n",
    );
    let flags: Vec<_> = s.types.iter().map(|t| (&*t.name, t.conditional)).collect();
    assert_eq!(
        flags,
        [("X", true), ("Inner", true), ("Top", false), ("T", true)]
    );
    let f: Vec<_> = s
        .functions
        .iter()
        .map(|f| (&*f.name, f.conditional))
        .collect();
    assert_eq!(f, [("f", false), ("g", true)]);
}

#[test]
fn php_dynamic_and_adapted_types_are_marked() {
    let s = php(
        "class M { public function __call($n, $a) {} }\nclass A { use T1, T2 { T1::f insteadof T2; } }\nclass H { public int $x { get => 1; } }\n",
    );
    assert!(s.types[0].dynamic);
    assert!(s.types[1].adapted);
    // A member kind the rows do not know leaves the member list possibly short.
    assert!(s.types[2].incomplete);
}

#[test]
fn php_calls_that_declare_names_are_listed() {
    let s = php(
        "define('FOO', 1);\n\\class_alias(A::class, 'B');\nclass_alias($x, $y);\nstrlen('x');\n",
    );
    let calls: Vec<_> = s.declaring_calls.iter().map(|c| &**c).collect();
    assert_eq!(
        calls,
        ["class_alias", "define"],
        "the dynamic alias and the constant"
    );
    let names: Vec<_> = s.types.iter().map(|t| (&*t.name, t.conditional)).collect();
    assert_eq!(
        names,
        [("B", true)],
        "a literal alias declares only its name"
    );
}

#[test]
fn php_anonymous_classes_declare_nothing_for_the_file() {
    let s = php("function f() { return new class { const A = 1; public function m() {} }; }\n");
    assert!(s.types.is_empty());
    assert!(s.constants.is_empty());
}

#[test]
fn a_file_with_syntax_errors_has_no_symbols() {
    let h = Harness::new();
    let file = h.file("a.php", "<?php\nclass A {\n");
    assert!(symbols(&h.db, file).as_ref().unwrap().is_none());
}

#[test]
fn a_language_without_symbol_rows_has_none() {
    let h = Harness::new();
    let file = h.file("a.yaml", "a: 1\n");
    assert!(symbols(&h.db, file).as_ref().unwrap().is_none());
}

#[test]
fn proto_package_qualifies_messages_enums_and_services() {
    let s = proto(
        "syntax = \"proto3\";\npackage foo.bar;\nmessage M { message N {} enum Q { Q_A = 0; } }\nenum E { E_A = 0; E_B = 1; }\nservice S { rpc Get(M) returns (stream .foo.bar.M); }\n",
    );
    let got: Vec<_> = s.types.iter().map(|t| (&*t.name, t.kind)).collect();
    assert_eq!(
        got,
        [
            ("foo.bar.M", TypeKind::Message),
            ("foo.bar.M.N", TypeKind::Message),
            ("foo.bar.M.Q", TypeKind::Enum),
            ("foo.bar.E", TypeKind::Enum),
            ("foo.bar.S", TypeKind::Service),
        ]
    );
    assert_eq!(
        members(&s.types[3]),
        [("E_A", MemberKind::Case), ("E_B", MemberKind::Case)]
    );
}

#[test]
fn proto_fields_are_members_through_oneofs_and_maps() {
    let s = proto(
        "syntax = \"proto3\";\nmessage M { string a = 1; repeated int32 b = 2; map<string, Z> m = 3; oneof k { int32 i = 4; } reserved 9; option deprecated = true; }\n",
    );
    assert_eq!(
        members(&s.types[0]),
        [
            ("a", MemberKind::Property),
            ("b", MemberKind::Property),
            ("m", MemberKind::Property),
            ("i", MemberKind::Property),
        ]
    );
    assert!(!s.types[0].incomplete);
    assert_eq!(&*s.types[0].name, "M");
}

#[test]
fn proto_rpc_types_resolve_through_the_enclosing_scopes() {
    let s = proto(
        "syntax = \"proto3\";\npackage foo.bar;\nservice S { rpc Get(M) returns (stream .x.Y); rpc Del(google.protobuf.Empty) returns (Empty); }\n",
    );
    let rpc =
        |i: usize| -> Vec<Vec<&str>> { s.types[0].members[i].refs.iter().map(names).collect() };
    assert_eq!(
        rpc(0),
        [vec!["foo.bar.S.M", "foo.bar.M", "foo.M", "M"], vec!["x.Y"]]
    );
    assert_eq!(rpc(1)[0][1], "foo.bar.google.protobuf.Empty");
    assert_eq!(s.types[0].members[0].kind, MemberKind::Method);
}

#[test]
fn proto_imports_are_files_with_their_modifier() {
    let s = proto(
        "syntax = \"proto3\";\nimport \"google/protobuf/empty.proto\";\nimport public \"other.proto\";\nimport weak \"w.proto\";\n",
    );
    let got: Vec<_> = s.imports.iter().map(|i| (&*i.path, i.public)).collect();
    assert_eq!(
        got,
        [
            ("google/protobuf/empty.proto", false),
            ("other.proto", true),
            ("w.proto", false),
        ]
    );
}

#[test]
fn an_edit_inside_a_body_leaves_the_symbols_equal_and_their_readers_alone() {
    let mut h = Harness::new();
    let text =
        "<?php\nnamespace A;\nclass C extends B {\n    public function m() { return 1; }\n}\n";
    let file = h.file("a.php", text);
    let before = symbols(&h.db, file).as_ref().unwrap().clone();
    assert_eq!(*type_count(&h.db, file), 1);
    assert_eq!((h.runs("symbols"), h.runs("type_count")), (1, 1));
    file.set_text(&mut h.db)
        .to(text.replace("return 1;", "return 2 + 3;\n\n    // more\n"));
    assert_eq!(*type_count(&h.db, file), 1);
    assert_eq!(*symbols(&h.db, file).as_ref().unwrap(), before);
    assert_eq!(
        (h.runs("symbols"), h.runs("type_count")),
        (2, 1),
        "extracted again, to an equal value: the reader did not run"
    );
    file.set_text(&mut h.db).to(text.replace("m()", "n()"));
    assert_ne!(*symbols(&h.db, file).as_ref().unwrap(), before);
}

#[test]
fn symbols_do_not_keep_the_parse_tree() {
    let h = Harness::new();
    h.symbols("a.php", "<?php\nclass A {}\n");
    assert_eq!(h.runs("symbols"), 1);
    assert_eq!(h.runs("parse("), 0, "the tree is dropped, not memoized");
}
