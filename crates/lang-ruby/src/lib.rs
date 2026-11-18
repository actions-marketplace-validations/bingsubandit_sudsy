//! Ruby: Prism parses behind `adapter`, which builds the lossless tree; `format.rs` lays the code
//! out.
//!
//! concept: none (glue: provider entry)

mod adapter;
mod format;
mod rules;

pub use adapter::parse;
pub use format::{RubyFormatter, option_schema};

include!("provider.generated.rs");
