//! Ruby: Prism parses behind `adapter`, which builds the lossless tree; the file is kept as written.
//!
//! concept: none (glue: provider entry)

mod adapter;
mod rules;

pub use adapter::parse;
use core_config::shared;

include!("schema.generated.rs");
include!("provider.generated.rs");
