//! Go: tree-sitter parses (through `core-treesitter`, mapping in `lang.yaml`); the file is kept as
//! written, because gofmt formats Go.
//!
//! concept: none (glue: provider entry)

mod rules;

use core_config::shared;

include!("schema.generated.rs");
include!("provider.generated.rs");
