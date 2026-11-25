//! The language provider interface and the out-of-process proxy that implements it for languages
//! parsed by a worker process (C#).
//!
//! concept: none (glue: crate root)

pub mod adapter;
pub mod directive_payload;
pub mod embedded;
pub mod encoding;
pub mod fixtures;
pub mod format;
pub mod format_testing;
pub mod guard;
pub mod out_of_process;
pub mod parser;
pub mod protocol;
pub mod provider;
pub mod proxy;
pub mod snippets;
pub mod testing;

pub use format::{
    FileFormatError, format_file, format_parsed, format_parsed_with, is_generated, resolve_dialect,
    select_provider, select_provider_index,
};
pub use out_of_process::OutOfProcessProvider;
pub use parser::{ParserProvider, ParserSpec};
pub use provider::{
    Capabilities, FileMatcher, FilePattern, LanguageProvider, ParseError, ParseResult,
    ProviderError, RUNTIME_MISSING, RUNTIME_URL, SourceError, SourceFile,
};
pub use proxy::{ProxyError, WorkerConfig, WorkerProxy};
pub use snippets::{FnSnippets, LazyRules, ProviderSnippets};
