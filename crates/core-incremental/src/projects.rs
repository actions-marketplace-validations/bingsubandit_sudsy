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
    for ((provider, root), mut files) in groups {
        // A fixed order, so that an unchanged set is not written again.
        files.sort_by(|a, b| a.path(db).cmp(b.path(db)));
        let covered = covered.iter().any(|r| root.starts_with(r));
        let old = current
            .iter()
            .find(|e| e.provider == provider && e.root == root);
        if let Some(old) = old {
            let mut entry = old.clone();
            // The same project keeps its memos: only a changed file list is set.
            if old.project.files(db) != &files {
                entry.left_out = left_out(db, provider, &root, &files, old.project.locator(db));
                old.project.set_files(db).to(files);
                changed |= entry.left_out != old.left_out;
            }
            let complete = covered && !entry.left_out;
            if old.project.complete(db) != complete {
                old.project.set_complete(db).to(complete);
            }
            list.push(entry);
            continue;
        }
        changed = true;
        let locator = core_packages::open(&systems[&provider].0, &root);
        let left = left_out(db, provider, &root, &files, &locator);
        let project = Project::new(db, files, covered && !left, locator, 0);
        list.push(ProjectEntry {
            provider,
            root,
            project,
            left_out: left,
        });
    }
    if changed {
        db.set_projects(list);
    }
}

/// Whether a file of the provider's language under `root` is not among the project's `files`
/// (left out by an ignore rule or the config), apart from the dependencies folder: such a file may
/// declare names, so the project cannot prove a name absent.
fn left_out(
    db: &Database,
    provider: usize,
    root: &Path,
    files: &[File],
    locator: &Option<std::sync::Arc<dyn core_packages::Locator>>,
) -> bool {
    let known: std::collections::HashSet<&Path> =
        files.iter().map(|f| f.path(db).as_path()).collect();
    let skip = locator.as_ref().and_then(|l| l.dependency_dir());
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                if entry.file_name() != ".git" && skip.as_deref() != Some(path.as_path()) {
                    stack.push(path);
                }
            } else if kind.is_file()
                && db.provider_index(&path) == Some(provider)
                && !is_manifest(db, &path)
                && !known.contains(path.as_path())
            {
                return true;
            }
        }
    }
    false
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

/// A manifest or a source file changed on disk: every project of a provider with that manifest
/// finds its package locations again, looks again for files the run left out, and reads the files
/// it loaded on demand again. The next `open_projects` sets `complete` from the result. Not seen:
/// a change in the files under the dependencies folder (`vendor/`) when the client does not report
/// it.
pub fn manifest_changed(db: &mut Database) {
    let list: Vec<ProjectEntry> = Projects::get(db).list(db).clone();
    let mut next_list = Vec::with_capacity(list.len());
    for mut entry in list {
        let p = db.providers()[entry.provider].as_ref();
        let Some(lang) = p.symbols() else {
            next_list.push(entry);
            continue;
        };
        let system = lang.table(p.grammar()).facts.packages.clone();
        let locator = core_packages::open(&system, &entry.root);
        let files = entry.project.files(db).clone();
        entry.left_out = left_out(db, entry.provider, &entry.root, &files, &locator);
        entry.project.set_locator(db).to(locator);
        let next = entry.project.revision(db) + 1;
        entry.project.set_revision(db).to(next);
        next_list.push(entry);
    }
    db.set_projects(next_list);
}

/// Whether `path` is a source file of a language that has a package system.
pub fn is_source(db: &Database, path: &Path) -> bool {
    db.provider_index(path).is_some_and(|i| {
        let p = db.providers()[i].as_ref();
        p.symbols()
            .is_some_and(|l| !l.table(p.grammar()).facts.packages.is_empty())
    })
}

/// File name globs of the source files of the languages that have a package system.
pub fn source_globs(db: &Database) -> Vec<String> {
    use provider_api::FilePattern;
    let mut out = Vec::new();
    for p in db.providers() {
        if !p
            .symbols()
            .is_some_and(|l| !l.table(p.grammar()).facts.packages.is_empty())
        {
            continue;
        }
        for m in p.file_matchers() {
            if let FilePattern::Extension(e) = &m.pattern {
                out.push(format!("**/*.{e}"));
            }
        }
    }
    out
}
