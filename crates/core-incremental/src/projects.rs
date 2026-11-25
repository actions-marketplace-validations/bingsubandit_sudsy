//! Opening the projects the index rules read: the files of a run grouped by the folder of their
//! language's manifest, each with its package locator. Nothing here runs unless a rule of one of
//! the run's languages reads the index.
//!
//! concept: none (glue: projects of a run)

use crate::db::{Database, Db};
use crate::inputs::{File, Project, ProjectEntry};
use crate::queries::selection;
use crate::rules::active_rules;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Whether an active rule of the file's language reads the index.
pub fn reads_index(db: &dyn Db, file: File) -> bool {
    active_rules(db, file)
        .as_ref()
        .is_ok_and(|(active, _)| active.iter().any(|a| a.rule.reads_index()))
}

/// The folder nearest above `path` that holds `manifest`.
fn project_root(
    path: &Path,
    manifest: &str,
    seen: &mut HashMap<PathBuf, Option<PathBuf>>,
) -> Option<PathBuf> {
    let dir = path.parent()?;
    if let Some(found) = seen.get(dir) {
        return found.clone();
    }
    let found = if dir.join(manifest).is_file() {
        Some(dir.to_path_buf())
    } else {
        project_root(dir, manifest, seen)
    };
    seen.insert(dir.to_path_buf(), found.clone());
    found
}

/// Opens one project for each manifest folder that holds files of a language whose rules read the
/// index (decided on the language's first file). A file under no manifest belongs to none.
pub fn open_projects(db: &mut Database, files: &[File]) {
    let mut wanted: HashMap<usize, bool> = HashMap::new();
    let mut groups: HashMap<(usize, PathBuf), Vec<File>> = HashMap::new();
    let mut roots: HashMap<usize, HashMap<PathBuf, Option<PathBuf>>> = HashMap::new();
    let mut systems: HashMap<usize, (String, String)> = HashMap::new();
    for &file in files {
        let Some(provider) = selection(db, file).as_ref().map(|s| s.provider) else {
            continue;
        };
        let reads = *wanted
            .entry(provider)
            .or_insert_with(|| reads_index(db, file));
        if !reads {
            continue;
        }
        let (system, manifest) = systems.entry(provider).or_insert_with(|| {
            let p = db.providers()[provider].as_ref();
            p.symbols()
                .map(|l| {
                    let facts = &l.table(p.grammar()).facts;
                    (facts.packages.clone(), facts.manifest.clone())
                })
                .unwrap_or_default()
        });
        if system.is_empty() {
            continue;
        }
        let seen = roots.entry(provider).or_default();
        if let Some(root) = project_root(file.path(db), manifest, seen) {
            groups.entry((provider, root)).or_default().push(file);
        }
    }
    let mut list = Vec::new();
    for ((provider, root), files) in groups {
        let locator = core_packages::open(&systems[&provider].0, &root);
        let project = Project::new(db, files, false, locator, 0);
        list.push(ProjectEntry {
            provider,
            root,
            project,
        });
    }
    db.set_projects(list);
}
