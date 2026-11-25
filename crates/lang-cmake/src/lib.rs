//! CMake (`CMakeLists.txt`, `*.cmake`): a hand-written lossless lexer and parser; the formatter
//! is the rows of `format.rules`.
//!
//! concept: none (glue: provider entry)

mod blocks;
mod format;
mod lexer;
mod parser;

use core_config::shared;

pub use blocks::BLOCKS;
pub use parser::parse;

include!("schema.generated.rs");
include!("provider.generated.rs");
