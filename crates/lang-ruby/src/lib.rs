//! Ruby: Prism parses behind `adapter`, which builds the lossless tree; `format` lays the file out by rows.
//!
//! concept: none (glue: provider entry)

mod adapter;
mod rules;

pub use adapter::parse;
use core_config::shared;

include!("schema.generated.rs");
include!("provider.generated.rs");
