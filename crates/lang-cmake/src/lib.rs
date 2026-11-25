//! CMake (`CMakeLists.txt`, `*.cmake`): a hand-written lossless lexer and parser; the formatter
//! prints the file as written.
//!
//! concept: none (glue: provider entry)

mod lexer;
mod parser;

use core_config::shared;

pub use parser::parse;

include!("schema.generated.rs");
include!("provider.generated.rs");
