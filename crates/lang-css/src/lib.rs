//! CSS, SCSS and Less: one hand-written lossless parser for the three dialects, and three
//! providers over it. `scss` and `less` take `css` as their config base.

core_syntax::include_generated!();
mod data;
mod format;
mod lexer;
mod parser;
mod rules;

pub use format::{CssFormatter, option_schema};
pub use parser::MAX_DEPTH;

use core_syntax::grammars;
use provider_api::{ParseResult, ParserProvider, ParserSpec, SourceFile};

/// The syntax a file is lexed with: `//` comments are SCSS and Less only, `$name` and `#{}` SCSS
/// only, `@{}` Less only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dialect {
    Css,
    Scss,
    Less,
}

impl Dialect {
    fn name(self) -> &'static str {
        match self {
            Dialect::Css => "css",
            Dialect::Scss => "scss",
            Dialect::Less => "less",
        }
    }
}

/// The provider of one dialect; `scss` and `less` take `css` as their config base. The three
/// share the rules; the pattern files use no snippets, so one parse function serves.
pub fn provider(dialect: Dialect) -> ParserProvider {
    static CSS: CssFormatter = CssFormatter::new(Dialect::Css);
    static SCSS: CssFormatter = CssFormatter::new(Dialect::Scss);
    static LESS: CssFormatter = CssFormatter::new(Dialect::Less);
    let formatter = match dialect {
        Dialect::Css => &CSS,
        Dialect::Scss => &SCSS,
        Dialect::Less => &LESS,
    };
    let spec = ParserSpec {
        crate_name: env!("CARGO_PKG_NAME"),
        grammar: grammars::CSS,
        extensions: match dialect {
            Dialect::Css => &["css", "uss"], // Unity UI Toolkit style sheets
            Dialect::Scss => &["scss"],
            Dialect::Less => &["less"],
        },
        file_names: &[],
        schema: option_schema(),
        formatter,
        hooks: &core_rules::DEFAULT_HOOKS,
        rules: &[],
        pattern_files: provider_api::pattern_files!(),
        families: rules::FAMILIES,
    };
    let provider = ParserProvider::new(spec, move |file| parse_file(file, dialect));
    match dialect {
        Dialect::Css => provider,
        _ => provider.with_dialect(dialect.name(), "css"),
    }
}

fn parse_file(file: &SourceFile, dialect: Dialect) -> ParseResult {
    // A host's declaration list (`EmbedMode::Declarations`) is the block of a rule.
    let text = match file.language_version.as_deref() {
        Some("declarations") => &format!(
            "a{{
{}
}}",
            file.text
        ),
        _ => &file.text,
    };
    let parsed = parser::parse(text, dialect);
    ParseResult {
        tree: parsed.tree,
        errors: parsed.errors,
    }
}
