//! What a file declares to other files (`core_semantics::symbols`) as a query. An
//! edit inside a body parses the file again and extracts again, but the symbols come out equal,
//! so nothing that reads them re-runs. The file is parsed here and the tree dropped at once, not
//! through the memoized `parse`: a project of thousands of files keeps their symbols, not their
//! trees. Nothing calls it unless a rule or a tool asks, so a run
//! without index rules reads no rows and walks no tree.
//!
//! concept: none (infrastructure: symbol query)

use crate::db::Db;
use crate::failure::{Failure, Stage, guarded};
use crate::inputs::File;
use crate::queries::{parse_settings, provider_of, source_file};
use core_semantics::symbols::{FileSymbols, extract};

/// The symbols of a file. `None` when the language declares no symbol rows, or the file has
/// syntax errors (what it declares is unknown, not empty).
#[salsa::tracked(returns(ref))]
pub fn symbols(db: &dyn Db, file: File) -> Result<Option<FileSymbols>, Failure> {
    guarded(Stage::Lint, || {
        let provider = provider_of(db, file)?;
        let Some(lang) = provider.symbols() else {
            return Ok(None);
        };
        let source = source_file(db, file, parse_settings(db, file));
        let parsed = provider_api::guard::parse(provider, &source)
            .map_err(|e| Failure::new(Stage::Parse, &e))?;
        if !parsed.errors.is_empty() {
            return Ok(None);
        }
        let grammar = provider.grammar();
        Ok(Some(extract(&parsed.tree, grammar, lang.table(grammar))))
    })
}
