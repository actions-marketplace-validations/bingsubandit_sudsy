//! The SQL provider (`.sql`): one hand-written parser (ansi, postgres, mysql and tsql dialects,
//! `parse`) builds the lossless tree; the formatter lays the file out itself (`layout`).
//!
//! concept: none (glue: provider entry)

mod format;
mod layout;
mod parse;
mod rules;

pub use format::{DIALECT, SqlFormatter, option_schema};
pub use parse::{DIALECTS, parse};

include!("provider.generated.rs");
