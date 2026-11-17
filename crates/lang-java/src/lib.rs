//! Java: rezel-lang-java parses behind `adapter`, which builds the lossless tree; `format/` lays
//! the code out like google-java-format.

mod adapter;
mod format;
mod imports;
mod lexer;
mod parser;
mod rules;

pub use adapter::parse;
pub use adapter::parse as parse_rezel;
pub use format::{JavaFormatter, option_schema};
pub use parser::parse as parse_handwritten;

include!("provider.generated.rs");
