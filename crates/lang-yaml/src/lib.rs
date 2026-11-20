//! The YAML provider: a hand-written lossless YAML 1.2 parser, the data model used by the
//! safety check, and the formatter.
//!
//! concept: none (glue: provider entry)

mod format;
mod hooks;
pub mod model;
mod parser;
mod quoted_strings;
mod resolve;
mod rules;
pub mod scalar;
mod tree;
mod value_rules;

pub use format::{
    COMMENT_SPACING, DOCUMENT_START, QUOTE_STYLE, SEQUENCE_INDENT, YamlFormatter, option_schema,
};
pub use parser::{MAX_DEPTH, Parsed, parse, parse_file};

include!("provider.generated.rs");
