//! INI-style files: tree-sitter parses (through `core-treesitter`, mapping in `lang.yaml`),
//! formatting is the shared sequence shape plus the section layout in `format.rs`.
//!
//! concept: none (glue: provider entry)

mod format;
mod rules;

pub use format::{IniFormatter, option_schema};

include!("provider.generated.rs");
