//! The formatter of CMake: the rows of `format.rules` over the block words of the language.
//!
//! concept: none (glue: the formatter's rows and facts)

use crate::blocks::BLOCKS;
use core_format::rules::LazyRules;
use core_syntax::grammars;

pub static ROWS: LazyRules =
    LazyRules::new(|| grammars::CMAKE.get(), include_str!("../format.rules")).blocks(&BLOCKS);
