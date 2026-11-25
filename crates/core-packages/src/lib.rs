//! Where the file of a name or an import lives: one [`Locator`] per package system (Composer's
//! autoload rules, a list of import path roots), built from the manifest files of a project and
//! answering by path arithmetic and a file-exists check, never by reading source. The answer says
//! whether the locator is complete for what was asked: only then does "no such file" mean the
//! name is absent; a name outside every known prefix is `Unknown`.
//!
//! concept: package-locator
//! what: Finds the file of a qualified type name or an import from a project's package manifests (Composer autoload, import path roots).
//! tools: PHPStan class.notFound, buf IMPORT_USED
//! reads: file:symbols.rows

mod composer;
mod import_paths;

pub use composer::Composer;
pub use import_paths::ImportPaths;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// What a locator knows about where something is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Located {
    /// These files exist and could hold it.
    Files(Vec<PathBuf>),
    /// The locator covers what was asked and no file is there: whatever the locator maps is not
    /// declared in a file of its own.
    Missing,
    /// What was asked is outside what the locator covers (no prefix, no root, no manifest).
    Unknown,
}

/// A package system's way of finding files.
pub trait Locator: Send + Sync {
    /// The files that may declare the type `name` (qualified, as the language writes it).
    fn type_files(&self, name: &str) -> Located;

    /// The file an import path names.
    fn import_file(&self, path: &str) -> Located;

    /// Files that declare names without a name-to-path rule (autoloaded function files, class
    /// maps). Read when a name is not found elsewhere.
    fn autoloaded(&self) -> Vec<PathBuf>;
}

/// The locator of the package system `system` (the `packages` fact of a language's rows) for the
/// project at `root`; `None` for a system this build does not know.
pub fn open(system: &str, root: &Path) -> Option<Arc<dyn Locator>> {
    match system {
        "composer" => Some(Arc::new(Composer::open(root))),
        "import-paths" => Some(Arc::new(ImportPaths::new(vec![root.to_path_buf()], false))),
        _ => None,
    }
}

/// `path` or, when only the case differs on a case-sensitive disk, the file the same name
/// spelled otherwise: a name that PHP compares without case must not become absent on Linux.
pub(crate) fn existing(path: &Path) -> Option<PathBuf> {
    if path.is_file() {
        return Some(path.to_path_buf());
    }
    let mut at = PathBuf::new();
    for part in path.components() {
        let next = at.join(part);
        if next.exists() {
            at = next;
            continue;
        }
        let want = part.as_os_str().to_string_lossy();
        let found = std::fs::read_dir(if at.as_os_str().is_empty() {
            Path::new(".")
        } else {
            &at
        })
        .ok()?
        .flatten()
        .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(&want))?;
        at = at.join(found.file_name());
    }
    at.is_file().then_some(at)
}
