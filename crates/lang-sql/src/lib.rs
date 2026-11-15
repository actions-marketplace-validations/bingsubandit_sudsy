//! The SQL provider (`.sql`): sqruff's parser (ansi, postgres, mysql and tsql dialects) behind
//! `adapter`, which builds the lossless tree; the formatter prints the file as written until the layout of our own lands.

mod adapter;
mod format;
mod rules;

pub use adapter::{DIALECTS, MAX_BYTES, parse};
pub use format::{DIALECT, SqlFormatter, option_schema};

include!("provider.generated.rs");
