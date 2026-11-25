//! The incremental query engine, built on `salsa`.
//!
//! Core computations are queries memoized by what they read, so an edit recomputes only what
//! depends on it. Today's inputs are file text ([`File`]) and the config files
//! ([`ConfigFile`]); today's queries are in [`queries`]: `config`, `selection`, `options`,
//! `parse_settings`, `parse` and `format`. A query that fails returns the failure as a value
//! ([`Failure`]).
//!
//! Backdating: when a query re-executes to an equal value, its dependents are not re-run.
//! Equality of every value in the graph is structural: `GreenNode` compares its tree, config and
//! options compare their contents, and failures compare their messages.
//!
//! concept: none (glue: crate root)

mod db;
mod embedded;
mod failure;
mod file_symbols;
mod fixing;
mod index;
mod inputs;
mod module;
mod queries;
mod rules;

pub use db::{Database, Db, EventHook, Providers};
pub use failure::{Failure, Stage};
pub use file_symbols::symbols;
pub use fixing::{fix_text, transient};
pub use index::{Answer, Found, Index};
pub use inputs::{ConfigFile, ExtensionSet, File, Project, WorkspaceRevision};
pub use module::{Siblings, Unit, directory_scope, module_diagnostics, module_facts};
pub use queries::{
    FileOptions, Parsed, Selection, config, format, options, parse, parse_settings, selection,
};
pub use rules::{
    LintSettings, Linted, layout, lint, lint_settings, project_diagnostics, semantics,
};
pub use salsa::Cancelled;
