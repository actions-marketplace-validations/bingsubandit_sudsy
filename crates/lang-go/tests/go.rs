use provider_api::format_testing::{
    assert_file_names, assert_formats, assert_lossless, assert_parses, bench_input_files,
    check_corpus,
};
use provider_api::{LanguageProvider, SourceFile};

/// Formatting keeps the file as written (gofmt formats Go), with every check of the shared helper.
#[track_caller]
fn fmt(input: &str) -> String {
    assert_formats(lang_go::provider(), "a.go", input)
}

/// Every `.go` file of the pinned corpus (`tests/conformance/fetch-go-corpus.sh`) parses, fits the
/// grammar and prints back as it was.
///
/// `cargo test --profile corpus -p lang-go --test integration -- --ignored --nocapture conformance`
#[test]
#[ignore]
fn conformance() {
    let (_, unparsed) = check_corpus(lang_go::provider(), "a.go", "go", &["go"]);
    assert!(
        unparsed.is_empty(),
        "valid files with syntax errors: {unparsed:#?}"
    );
}

#[test]
fn file_names() {
    assert_file_names(
        &lang_go::provider(),
        &["a.go", "pkg/a_test.go", "zz_generated.deepcopy.go"],
        &["a.gomod", "go.mod", "a.py"],
    );
}

#[test]
fn bench_inputs_parse() {
    for (name, text) in bench_input_files("go") {
        assert_parses(&lang_go::provider(), &name, &text);
    }
}

#[test]
fn files_are_kept_as_written() {
    let src = "package a\n\nimport \"fmt\"\n\nfunc  f( x int )  {\n\tfmt.Println( x )\n}\n";
    assert_eq!(fmt(src), src);
}

/// Go 1.26 `new(expr)`: the upstream grammar takes only a type as the first argument of `new`.
#[test]
fn new_takes_an_expression() {
    assert_parses(
        &lang_go::provider(),
        "a.go",
        "package a\n\nfunc f() {\n\ta := new(int32(1))\n\tb := new(\"a\")\n\tc := new(T)\n\td := new(max(x, y))\n\te := new(2 * time.Second)\n\tf := make([]int, n)\n}\n",
    );
}

/// Generic methods (Go 1.27): a method name takes type parameters.
#[test]
fn methods_take_type_parameters() {
    assert_parses(
        &lang_go::provider(),
        "a.go",
        "package a\n\nfunc (r *Rand) N[Int intType](n Int) Int {\n\treturn n\n}\n\nfunc (t T[P]) G[R any](buf []byte) R {\n\tvar r R\n\treturn r\n}\n",
    );
}

/// Go syntax the grammar must take: generics, select, labels, goto, struct tags, raw strings,
/// composite literals, channels, closures, iota, type switches, all numeric literals.
#[test]
fn go_syntax_parses() {
    assert_parses(
        &lang_go::provider(),
        "a.go",
        r#"// Package a does things.
package a

import (
	"fmt"
	. "math"
	_ "embed"
	str "strings"
)

type (
	Number interface {
		~int | ~int64 | float64
	}
	Pair[K comparable, V any] struct {
		Key   K `json:"key"`
		Value V `json:"value,omitempty"`
		sync.Mutex
	}
	Alias = map[string][]*Pair[string, int]
	Fn    func(a, b int, rest ...string) (int, error)
)

const (
	A = iota
	B
	C = 1 << iota
)

var x, y = 0x1F, 0b101 + 0o17 + 1_000 + 1.5e3 + 2i + 'a'

func Map[T, U any](s []T, f func(T) U) []U {
	out := make([]U, 0, len(s))
	for i, v := range s {
		out = append(out, f(v))
	}
	for range 10 {
	}
	for i := range 3 {
		_ = i
	}
	return out
}

func (p *Pair[K, V]) String() string { return fmt.Sprint(p.Key, p.Value) }

func main() {
	ch := make(chan<- int, 1)
	var rc <-chan int
	select {
	case v, ok := <-rc:
		_, _ = v, ok
	case ch <- 1:
	default:
	}
outer:
	for {
		switch v := any(x).(type) {
		case int, string:
			break outer
		case nil:
			fallthrough
		default:
			_ = v
		}
		goto outer
	}
	defer func() { recover() }()
	go func(a int) {}(1)
	m := map[string]struct{ a int }{"k": {a: 1}}
	s := []int{1, 2, 3}[1:2:3]
	p := &Pair[string, int]{Key: "k"}
	p.Value++
	x <<= 2
	raw := `a
b`
	_, _, _, _ = m, s, raw, str.ToUpper("x")
	if v := f(); v > 0 && !ok {
	} else if v < 0 {
	} else {
	}
	;
}
"#,
    );
}

#[test]
fn syntax_errors_stay_lossless() {
    let parsed = assert_lossless(
        &lang_go::provider(),
        "a.go",
        "package a\n\nfunc f( {\n\tx := \n}\n",
    );
    assert!(!parsed.errors.is_empty());
}

/// Parse speed over each repository of the corpus, whole files from memory, best of 3.
///
/// `cargo test --release -p lang-go --test integration -- --ignored --nocapture speed`
#[test]
#[ignore = "a measurement"]
fn speed() {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance/corpus/go");
    let provider = lang_go::provider();
    for repo in std::fs::read_dir(root).unwrap() {
        let mut texts = Vec::new();
        let mut dirs = vec![repo.unwrap().path()];
        while let Some(d) = dirs.pop() {
            for e in std::fs::read_dir(d).unwrap() {
                let path = e.unwrap().path();
                if path.is_dir() {
                    dirs.push(path);
                } else if let Ok(text) = std::fs::read_to_string(&path) {
                    texts.push(text);
                }
            }
        }
        let bytes: usize = texts.iter().map(String::len).sum();
        let best = (0..3)
            .map(|_| {
                let start = std::time::Instant::now();
                for t in &texts {
                    std::hint::black_box(provider.parse(&SourceFile::new("a.go", 1, t)).unwrap());
                }
                start.elapsed()
            })
            .min()
            .unwrap();
        println!(
            "{:?}: {} files, {:.1} MB, {:.1} MB/s",
            best,
            texts.len(),
            bytes as f64 / 1e6,
            bytes as f64 / 1e6 / best.as_secs_f64()
        );
    }
}
