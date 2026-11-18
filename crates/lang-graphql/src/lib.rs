//! GraphQL (executable documents and schema definition language): tree-sitter parses (through
//! `core-treesitter`, mapping in `lang.yaml`), `format.rs` lays the definitions out like Prettier.
//!
//! concept: none (glue: provider entry)

mod format;
mod rules;

pub use format::{GraphqlFormatter, option_schema};

include!("provider.generated.rs");
