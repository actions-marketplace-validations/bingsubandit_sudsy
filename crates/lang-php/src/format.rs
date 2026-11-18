//! PHP formatting is data: `format.rules` (docs/formatting/php.md).

use core_config::shared;
use core_format::rules::LazyRules;
use core_syntax::grammars;

include!("schema.generated.rs");

#[allow(non_upper_case_globals)]
pub static PhpFormatter: LazyRules =
    LazyRules::new(|| grammars::PHP.get(), include_str!("../format.rules"));
