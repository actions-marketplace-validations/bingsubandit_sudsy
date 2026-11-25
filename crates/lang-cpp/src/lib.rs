//! C and C++ (`.c .h`, `.cc .cpp .cxx .hpp .hh .hxx .ipp`): read without preprocessing, as
//! clang-format reads them. `lexer` gives every token, `structure` the unwrapped lines, bracket
//! groups, directives and conditional groups, `deep` the statements, declarations and expressions
//! inside them; the two providers (`cpp`, and `c` as its dialect)
//! share both. The formatter prints the file as written.
//!
//! concept: none (glue: provider entry)

core_syntax::include_generated!();
mod deep;
mod format;
mod lexer;
mod payload;
mod structure;

pub use format::{CppFormatter, option_schema};

use core_syntax::grammars;
use provider_api::adapter::{Tok, replay_with};
use provider_api::directive_payload::merge_nodes;
use provider_api::{ParseError, ParseResult, ParserProvider, ParserSpec, SourceFile};

/// The keywords a file is lexed with: C++ words such as `class`, `new` and `this` are names in C.
/// `CFile` is a `.c` file, which is certainly C (no literal suffix after a string); `C` may be a
/// header of C++.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dialect {
    C,
    CFile,
    Cpp,
}

/// The provider of a dialect; `c` takes `cpp` as its config base. `.h` is C: a project whose
/// headers are C++ writes its `cpp` options where the C ones would apply, or names the files.
pub fn provider(dialect: Dialect) -> ParserProvider {
    let spec = ParserSpec {
        crate_name: env!("CARGO_PKG_NAME"),
        grammar: grammars::CPP,
        extensions: match dialect {
            Dialect::C | Dialect::CFile => &["c", "h"],
            Dialect::Cpp => &["cc", "cpp", "cxx", "hpp", "hh", "hxx", "ipp"],
        },
        file_names: &[],
        schema: option_schema(),
        formatter: &CppFormatter,
        hooks: &core_rules::DEFAULT_HOOKS,
        rules: &[],
        pattern_files: provider_api::pattern_files!(),
        families: &[],
    };
    let provider =
        ParserProvider::new(spec, move |file| parse_file(file, dialect)).with_format_parse();
    match dialect {
        Dialect::Cpp => provider,
        Dialect::C | Dialect::CFile => provider.with_dialect("c", "cpp"),
    }
}

fn parse_file(file: &SourceFile, dialect: Dialect) -> ParseResult {
    let certain = dialect == Dialect::C && file.id.ends_with(".c");
    let dialect = if certain { Dialect::CFile } else { dialect };
    parse_with(&file.text, dialect, file.for_format)
}

pub fn parse(src: &str, dialect: Dialect) -> ParseResult {
    parse_with(src, dialect, false)
}

/// The tree of `src`; for formatting, the code of the directives is read as well.
fn parse_with(src: &str, dialect: Dialect, for_format: bool) -> ParseResult {
    let lexed = lexer::lex(src, dialect);
    let mut nodes = deep::build(src, &lexed.toks, structure::build(src, &lexed.toks));
    if for_format {
        nodes = merge_nodes(nodes, payload::nodes(src, &lexed.toks));
    }
    let tokens: Vec<Tok> = (lexed.toks.iter())
        .map(|t| Tok {
            kind: t.kind,
            start: t.start,
            end: t.end,
        })
        .collect();
    let mut result = replay_with(src, grammars::CPP.get(), true, &tokens, &nodes, |t, _| t);
    let errors = (lexed.errors.iter()).map(|&(at, len, msg)| ParseError::syntax(at, len, msg));
    result.errors.extend(errors);
    result.errors.sort_by_key(|e| e.start);
    result
}
