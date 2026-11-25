use provider_api::format_testing::{
    assert_file_names, assert_formats, assert_lossless, assert_parses, bench_input_files,
    check_corpus,
};

/// Formats with every check of the shared helper (round trip, meaning, idempotency, comments).
#[track_caller]
fn fmt(input: &str) -> String {
    assert_formats(lang_ruby::provider(), "a.rb", input)
}

/// Every `.rb` and `.rake` file of the pinned corpus (`tests/conformance/fetch-ruby-corpus.sh`)
/// parses, fits the grammar, formats with the same tokens (the safety check) and idempotently.
///
/// `cargo test --profile corpus -p lang-ruby --test integration -- --ignored --nocapture conformance`
#[test]
#[ignore]
fn conformance() {
    let (_, unparsed) = check_corpus(lang_ruby::provider(), "a.rb", "ruby", &["rb", "rake"]);
    assert!(
        unparsed.is_empty(),
        "valid files with syntax errors: {unparsed:#?}"
    );
}

#[test]
fn file_names() {
    assert_file_names(
        &lang_ruby::provider(),
        &[
            "a.rb",
            "lib/a.rake",
            "a.gemspec",
            "config.ru",
            "Gemfile",
            "app/Rakefile",
        ],
        &["a.rbs", "a.py", "Gemfile.lock"],
    );
}

#[test]
fn bench_inputs_parse() {
    for (name, text) in bench_input_files("rb") {
        assert_parses(&lang_ruby::provider(), &name, &text);
    }
}

/// Ruby 3.x syntax: pattern matching, endless methods, `it`, hash shorthand, anonymous
/// forwarding, heredocs (squiggly, stacked, with calls after the opener), percent literals,
/// character literals, rational and imaginary numbers, `BEGIN`/`END`, flip-flops.
#[test]
fn ruby_3_syntax_parses() {
    assert_parses(
        &lang_ruby::provider(),
        "a.rb",
        r#"# frozen_string_literal: true
module A::B
  class C < D
    include E
    attr_reader :x, :"y z"

    def initialize(x, y = 1, *rest, k:, o: 2, **opts, &blk) = super
    def hi(n) = puts "hi #{n}"
    def fwd(*, **, &) = other(*, **, &)
    def all(...) = other(...)
    def ==(other) = x == other.x
    def self.[](i) = new(i)

    def run
      x = 1
      case [1, [2, 3]]
      in [Integer => a, [b, *c]] if a > 0 then a
      in {name: String => n, **rest} then n
      in ^x | nil then 0
      in [*, 3, *] then 1
      end
      x => {y:}
      h = {x:, y:, "k": 1, 'z' => 2, **opts}
      [1, 2].map { it * 2 }.each { _1 + 1 }
      a = <<~ONE + <<-TWO
        one #{x + 1}
      ONE
        two
        TWO
      call(<<~EOS, 1)
        body
      EOS
      %w[a b#{c} d].each { |w; v| puts w }
      %i[a b] + %q(x) + %Q(#{y}) + %r{a/b}i
      ?a + 1r + 2i + 3ri + 0x1f + 1_000
      puts(x) if y unless z
      begin; foo; rescue A, B => e; retry; else bar; ensure baz; end
      -> (a, b = 2) { a + b }.(1)
      a&.b&.c ||= d::E
      a, (b, *c), d = *f
      while x do y end
      for i in 1..3 do i end
      x = (1..)
      puts [1, 2,
            3]
        .sum
      obj.foo = 1
      obj[1] += 2
      alias_method :a, :b
      alias c d
      __FILE__
    end
  end
end
BEGIN { 1 }
END { 2 }
=begin
block comment
=end
__END__
data after the end
"#,
    );
}

/// The text after `__END__` is data: a file that ends in it keeps its end byte for byte.
#[test]
fn data_after_end_is_kept() {
    for s in [
        "puts 1\n__END__\nraw # data\n\n",
        "puts 1\n__END__",
        "puts 1\n__END__\n",
        "puts 1\n__END__\n\n\n",
    ] {
        assert_eq!(fmt(s), s);
    }
}

/// Prism spans a `rescue` clause over the clauses after it; they must be siblings.
#[test]
fn rescue_clauses_are_siblings() {
    let s = "def a\n  b\nrescue A\n  c\nrescue B\n  d\nrescue\n  e\nend\n";
    assert_lossless(&lang_ruby::provider(), "a.rb", s);
    assert_eq!(fmt(s), s);
}

#[test]
fn syntax_errors_stay_lossless() {
    let parsed = assert_lossless(
        &lang_ruby::provider(),
        "a.rb",
        "class A\n  def f(\nend end\n",
    );
    assert!(!parsed.errors.is_empty());
}

/// The root is the only `Program`: a check registered on `Program` runs once.
#[test]
fn the_tree_has_one_program_node() {
    let parsed = lang_ruby::parse("a = 1\nputs a\n");
    let g = core_syntax::grammars::RUBY.get();
    let programs = parsed
        .tree
        .descendants()
        .filter(|n| g.name(n.kind()) == "Program")
        .count();
    assert_eq!(programs, 1);
}

/// Parse speed over each repository of the corpus, whole files from memory, best of 3.
///
/// `cargo test --release -p lang-ruby --test integration -- --ignored --nocapture speed`
#[test]
#[ignore = "a measurement"]
fn speed() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/conformance/corpus/ruby");
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
                    std::hint::black_box(lang_ruby::parse(t));
                }
                start.elapsed()
            })
            .min()
            .unwrap();
        println!(
            "{} files, {:.1} MB: {:.0} ms, {:.1} MB/s",
            texts.len(),
            bytes as f64 / 1e6,
            best.as_secs_f64() * 1e3,
            bytes as f64 / 1e6 / best.as_secs_f64()
        );
    }
}

/// Indentation and line breaks stay as written; blank lines collapse to one and the blanks inside
/// brackets, around commas and in front of a `=` follow RuboCop's Layout cops.
#[test]
fn layout_rows() {
    assert_eq!(
        fmt("a=1\n\n\n\nb = [ 1,2 , 3 ]\n"),
        "a = 1\n\nb = [1, 2, 3]\n"
    );
    assert_eq!(
        fmt("x = { a: 1 }\ny = {a: 1}\nz = { }\n"),
        "x = { a: 1 }\ny = { a: 1 }\nz = {}\n"
    );
    assert_eq!(fmt("f( a, b )\n  g { |x|x }\n"), "f(a, b)\n  g { |x|x }\n");
    // Blanks that line up with a neighbouring line stay.
    let aligned = "a    = 1\nbbbb = 2\nfoo :a,    1 # c\n";
    assert_eq!(fmt(aligned), aligned);
}

/// A comment after a heredoc keeps its indentation, and a token that spans lines is not touched.
#[test]
fn heredoc_bodies_and_comments_stay() {
    let s = "def f\n  x = <<~P\n    a  \n\n\n    b\n  P\n  # c\n  g(x)\nend\n";
    assert_eq!(fmt(s), s);
}

/// Blank lines at the start of the file stay.
#[test]
fn leading_blank_lines_stay() {
    assert_eq!(fmt("\n\n# c\nx\n"), "\n\n# c\nx\n");
}
