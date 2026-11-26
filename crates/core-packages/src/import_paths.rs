//! The import path roots of a project: an import names a file below one of them.
//!
//! concept: package-locator
//! what: Finds the file of a qualified type name or an import from a project's package manifests (Composer autoload, import path roots).
//! tools: PHPStan class.notFound, buf IMPORT_USED
//! reads: file:symbols.rows

use super::{Located, Locator};
use std::path::PathBuf;

pub struct ImportPaths {
    roots: Vec<PathBuf>,
    /// The roots are all there are (named by the user); only then is a path that no root holds
    /// missing. Otherwise another root may exist, and the answer is unknown.
    complete: bool,
}

impl ImportPaths {
    pub fn new(roots: Vec<PathBuf>, complete: bool) -> ImportPaths {
        ImportPaths { roots, complete }
    }
}

impl Locator for ImportPaths {
    fn type_files(&self, _name: &str) -> Located {
        Located::Unknown
    }

    fn import_file(&self, path: &str) -> Located {
        let found: Vec<PathBuf> = self
            .roots
            .iter()
            .map(|r| r.join(path))
            .filter(|p| p.is_file())
            .take(1)
            .collect();
        if !found.is_empty() {
            Located::Files(found)
        } else if self.complete {
            Located::Missing
        } else {
            Located::Unknown
        }
    }

    fn autoloaded(&self) -> Vec<PathBuf> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_import_is_found_below_the_first_root_that_holds_it() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(b.path().join("x")).unwrap();
        std::fs::write(b.path().join("x/y.proto"), "").unwrap();
        let roots = vec![a.path().to_path_buf(), b.path().to_path_buf()];
        let open = ImportPaths::new(roots.clone(), false);
        let closed = ImportPaths::new(roots, true);
        let found = Located::Files(vec![b.path().join("x/y.proto")]);
        assert_eq!(open.import_file("x/y.proto"), found);
        assert_eq!(closed.import_file("x/y.proto"), found);
        // Another root may exist unless the roots are all there are.
        assert_eq!(open.import_file("x/z.proto"), Located::Unknown);
        assert_eq!(closed.import_file("x/z.proto"), Located::Missing);
        assert_eq!(open.type_files("p.M"), Located::Unknown);
    }
}
