//! What an index rule sees of one file: the names the file writes and the answers of the project's
//! index about them (`core_rules::index_rules::IndexView`). The project of a file is the one
//! whose root is its nearest enclosing folder with the language's manifest; a file outside every
//! project has no view, so its index rules report nothing.
//!
//! concept: none (glue: the rules' view of the index)

use crate::db::Db;
use crate::index::{Answer, Index, file_symbols, table_of};
use crate::inputs::{File, Project, Projects};
use crate::queries::selection;
use core_rules::index_rules::{IndexView, Referrer};
use core_semantics::symbols::{FileSites, MemberKind, Name, extract_sites};
use core_syntax::SyntaxNode;
use std::collections::HashMap;

/// The project of `file`: the entry of its language with the longest root above the file.
fn project_of(db: &dyn Db, file: File) -> Option<Project> {
    let provider = selection(db, file).as_ref()?.provider;
    let path = file.path(db);
    Projects::get(db)
        .list(db)
        .iter()
        .filter(|e| e.provider == provider && path.starts_with(&e.root))
        .max_by_key(|e| e.root.as_os_str().len())
        .map(|e| e.project)
}

pub(crate) struct FileIndex<'db> {
    db: &'db dyn Db,
    project: Project,
    file: File,
    sites: FileSites,
}

/// The view for `file` (its tree `root` is what the rules read), when it belongs to a project.
pub(crate) fn file_index<'db>(
    db: &'db dyn Db,
    file: File,
    root: &SyntaxNode,
) -> Option<FileIndex<'db>> {
    let project = project_of(db, file)?;
    let provider = crate::queries::provider_of(db, file).ok()?;
    let grammar = provider.grammar();
    let sites = extract_sites(root, grammar, provider.symbols()?.table(grammar));
    Some(FileIndex {
        db,
        project,
        file,
        sites,
    })
}

impl IndexView for FileIndex<'_> {
    fn sites(&self) -> &FileSites {
        &self.sites
    }

    fn import_declares(&self, path: &str, names: &[Name]) -> Answer<bool> {
        Index::new(self.db, self.project).import_declares(self.file, path, names)
    }

    fn resolve(&self, name: &Name) -> Answer<Box<str>> {
        Index::new(self.db, self.project).resolve(self.file, name)
    }

    fn referrers(&self, target: &str) -> Vec<Referrer> {
        method_targets(self.db, self.project)
            .get(target)
            .cloned()
            .unwrap_or_default()
    }
}

/// The methods of the project's files by the qualified type they name.
#[salsa::tracked(returns(ref))]
fn method_targets(db: &dyn Db, project: Project) -> HashMap<Box<str>, Vec<Referrer>> {
    let index = Index::new(db, project);
    let mut targets: HashMap<Box<str>, Vec<Referrer>> = HashMap::new();
    for &file in project.files(db) {
        let Some(symbols) = file_symbols(db, file) else {
            continue;
        };
        if table_of(db, file).is_none() {
            continue;
        }
        for ty in &symbols.types {
            for member in ty.members.iter().filter(|m| m.kind == MemberKind::Method) {
                for name in &member.refs {
                    if let Answer::Known(target) = index.resolve(file, name) {
                        targets.entry(target).or_default().push(Referrer {
                            path: file.path(db).clone(),
                            owner: ty.name.clone(),
                            method: member.name.clone(),
                        });
                    }
                }
            }
        }
    }
    targets
}
