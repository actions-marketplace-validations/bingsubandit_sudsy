//! PowerShell: tree-sitter parses (through `core-treesitter`, mapping in `lang.yaml`), the
//! formatter in `format.rs` re-indents the lines and adds the blanks and breaks
//! PSScriptAnalyzer's `Invoke-Formatter` adds.
//!
//! concept: none (glue: provider entry)

mod format;
mod hooks;
mod rules;
mod commands {
    include!("commands.generated.rs");
}
mod scope;

pub use format::{PowerShellFormatter, option_schema};

include!("provider.generated.rs");
