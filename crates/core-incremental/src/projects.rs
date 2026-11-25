//! Opening the projects the index rules read: the files of a run grouped by the folder of their
//! language's manifest, each with its package locator. Nothing here runs unless a rule of one of
//! the run's languages reads the index.
//!
//! concept: none (glue: projects of a run)

use crate::db::{Database, Db};
use crate::inputs::{File, Project, ProjectEntry, Projects};
use crate::queries::selection;
use crate::rules::active_rules;
use salsa::Setter;
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
/// index (decided on the language's first file). A file under no manifest belongs to none. `covered`
/// are the folders whose files were all read: a project below one of them holds every file of its
/// folder (`Project::complete`), the only case where a name no file declares can be absent.
pub fn open_projects(db: &mut Database, files: &[File], covered: &[PathBuf]) {
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
    let current: Vec<ProjectEntry> = Projects::get(db).list(db).clone();
    let mut list = Vec::new();
    let mut changed = current.len() != groups.len();
    for ((provider, root), files) in groups {
        let complete = covered.iter().any(|r| root.starts_with(r));
        let old = current
            .iter()
            .find(|e| e.provider == provider && e.root == root);
        if let Some(old) = old {
            // The same project keeps its memos: only a changed file list is set.
            if old.project.files(db) != &files {
                old.project.set_files(db).to(files);
            }
            if old.project.complete(db) != complete {
                old.project.set_complete(db).to(complete);
            }
            list.push(old.clone());
            continue;
        }
        changed = true;
        let locator = core_packages::open(&systems[&provider].0, &root);
        let project = Project::new(db, files, complete, locator, 0);
        list.push(ProjectEntry {
            provider,
            root,
            project,
        });
    }
    if changed {
        db.set_projects(list);
    }
}

/// Whether `path` is the manifest of some provider's package system (`composer.json`).
pub fn is_manifest(db: &Database, path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    manifests(db).iter().any(|m| m == name)
}

/// The manifest file names of the providers that have a package system.
pub fn manifests(db: &Database) -> Vec<String> {
    db.providers()
        .iter()
        .filter_map(|p| {
            let facts = &p.symbols()?.table(p.grammar()).facts;
            (!facts.packages.is_empty()).then(|| facts.manifest.clone())
        })
        .collect()
}

/// A manifest changed on disk: every project of a provider with that manifest finds its package
/// locations again and reads the files it loaded on demand again. Not seen: a change in the files
/// under the dependencies folder (`vendor/`), which no manifest names.
pub fn manifest_changed(db: &mut Database) {
    let list: Vec<ProjectEntry> = Projects::get(db).list(db).clone();
    for entry in list {
        let p = db.providers()[entry.provider].as_ref();
        let Some(lang) = p.symbols() else { continue };
        let system = lang.table(p.grammar()).facts.packages.clone();
        let locator = core_packages::open(&system, &entry.root);
        entry.project.set_locator(db).to(locator);
        let next = entry.project.revision(db) + 1;
        entry.project.set_revision(db).to(next);
    }
}
