//! What a file declares to other files (`core_semantics::symbols`) as a query of its parse. An
//! edit inside a body re-parses the file and extracts again, but the symbols come out equal, so
//! nothing that reads them re-runs. Nothing calls it unless a rule or a tool asks, so a run
//! without index rules reads no rows and walks no tree.
//!
//! concept: none (infrastructure: symbol query)

use crate::db::Db;
use crate::failure::{Failure, Stage, guarded};
use crate::inputs::File;
use crate::queries::{parse, provider_of};
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
        let parsed = parse(db, file).as_ref().map_err(Clone::clone)?;
        if !parsed.errors.is_empty() {
            return Ok(None);
        }
        let grammar = provider.grammar();
        Ok(Some(extract(&parsed.root(), grammar, lang.table(grammar))))
    })
}
