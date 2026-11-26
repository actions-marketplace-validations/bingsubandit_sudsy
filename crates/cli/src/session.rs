//! What every command that works on files shares: the project, the database with its config and
//! the files to process (discovered, filtered by the config's include and exclude).

use anyhow::{Context, Result};
use core_config::Depth;
use core_incremental::{Database, File, open_projects, reads_index};
use languages::{Project, providers};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct Session {
    pub project: Project,
    pub db: Database,
    /// Absolute paths, in the order they are reported.
    pub paths: Vec<PathBuf>,
    /// The folders and files named on the command line (the working directory when none), absolute.
    roots: Vec<PathBuf>,
}

impl Session {
    /// One database per run: the config is an input, and so is every file, created by the thread
    /// that reads it. `depth` is the command line's `--analysis`, which beats the config.
    pub fn open(args: &[PathBuf], depth: Option<Depth>) -> Result<Session> {
        let providers = providers();
        let mut project = Project::load(&std::env::current_dir()?, &providers)?;
        if let Some(depth) = depth {
            project.chain = project.chain.clone().with_depth(depth);
        }
        let mut db = Database::new(providers);
        db.set_config(Ok(project.chain.clone()));
        db.set_extensions(project.extensions.clone());
        // The files semantic rules bind against: what was named on the command line, else the
        // working directory.
        let roots = if args.is_empty() {
            vec![std::env::current_dir()?]
        } else {
            args.to_vec()
        };
        let filter = project.filter();
        let found = core_workspace::discover_files(&roots, &filter)?;
        db.open_project(&roots, &found);
        let paths = core_workspace::discover_files(args, &filter)?
            .into_iter()
            .filter(|path| db.supports(path))
            .collect();
        #[cfg(feature = "pack-mode")]
        let paths = crate::missing::skip(&db, &project, paths);
        let roots = roots
            .iter()
            .map(|r| std::path::absolute(r).unwrap_or_else(|_| r.clone()))
            .collect();
        Ok(Session {
            project,
            db,
            paths,
            roots,
        })
    }

    /// Reads every file into the database and runs `warm` on it, in parallel; the results of the
    /// queries `warm` runs stay memoized for the caller. Returned in `paths` order.
    pub fn load_all(&self, warm: impl Fn(&Database, File) + Sync) -> Vec<Result<File>> {
        let project = &self.project;
        let loaded = par_map(&self.db, &self.paths, |db, path| {
            let file = load(db, project, path)?;
            warm(db, file);
            Ok(file)
        });
        self.warn_ignored_editorconfig_keys(&loaded);
        loaded
    }

    /// [`Session::load_all`] for the commands that lint. When a rule of one of the languages reads
    /// the symbol index, every file is read first, the projects are opened, and `warm` runs then,
    /// so that a rule sees all files of its project; otherwise one pass reads and warms each file.
    pub fn load_linted(&mut self, warm: impl Fn(&Database, File) + Sync) -> Vec<Result<File>> {
        if !self.index_wanted() {
            return self.load_all(warm);
        }
        let loaded = self.load_all(|_, _| {});
        let files: Vec<File> = loaded
            .iter()
            .filter_map(|f| f.as_ref().ok().copied())
            .collect();
        open_projects(&mut self.db, &files, &self.roots);
        par_map(&self.db, &files, |db, file| warm(db, *file));
        loaded
    }

    /// Whether a rule of the language of some path reads the index, decided on the first path of
    /// each language.
    fn index_wanted(&self) -> bool {
        let mut probed = Vec::new();
        for path in &self.paths {
            let Some(provider) = self.db.provider_index(path) else {
                continue;
            };
            if probed.contains(&provider) {
                continue;
            }
            probed.push(provider);
            if load(&self.db, &self.project, path).is_ok_and(|f| reads_index(&self.db, f)) {
                return true;
            }
        }
        false
    }

    /// One stderr warning per `.editorconfig` line that sets a C# formatting key Sudsy does not
    /// follow, however many files it applies to.
    fn warn_ignored_editorconfig_keys(&self, loaded: &[Result<File>]) {
        let mut seen = std::collections::HashSet::new();
        for (path, file) in self.paths.iter().zip(loaded) {
            let Ok(file) = file else { continue };
            if path.extension().is_none_or(|e| e != "cs" && e != "csx") {
                continue;
            }
            for ignored in &file.editorconfig(&self.db).ignored {
                if seen.insert((ignored.file.clone(), ignored.line)) {
                    eprintln!(
                        "warning: {}:{}: `{} = {}` is not supported by Sudsy and is ignored",
                        self.project.show(&ignored.file),
                        ignored.line,
                        ignored.key,
                        ignored.value
                    );
                }
            }
        }
    }
}

/// Reads one file into the database.
pub fn load(db: &Database, project: &Project, path: &Path) -> Result<File> {
    let bytes = std::fs::read(path).context("reading the file")?;
    Ok(File::from_bytes(
        db,
        path.to_path_buf(),
        project.show(path),
        1,
        bytes,
    )?)
}

/// Runs `work` on every path on all cores. Inputs are only ever created, never set, while the
/// workers run: each works through its own handle on the database (a salsa handle shares the
/// memos) and claims the next path from a shared counter. Results come back in `paths` order.
pub fn par_map<I: Sync, T: Send>(
    db: &Database,
    paths: &[I],
    work: impl Fn(&Database, &I) -> T + Sync,
) -> Vec<T> {
    let threads = std::thread::available_parallelism()
        .map_or(1, usize::from)
        .min(paths.len());
    let next = AtomicUsize::new(0);
    let mut slots: Vec<Option<T>> = paths.iter().map(|_| None).collect();
    std::thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                let db = db.clone();
                let (next, work) = (&next, &work);
                scope.spawn(move || {
                    let mut done = Vec::new();
                    loop {
                        let index = next.fetch_add(1, Ordering::Relaxed);
                        let Some(path) = paths.get(index) else {
                            break done;
                        };
                        done.push((index, work(&db, path)));
                    }
                })
            })
            .collect();
        for worker in workers {
            match worker.join() {
                Ok(done) => {
                    for (index, result) in done {
                        slots[index] = Some(result);
                    }
                }
                Err(panic) => std::panic::resume_unwind(panic),
            }
        }
    });
    slots
        .into_iter()
        .map(|slot| slot.expect("every path is claimed by one worker"))
        .collect()
}
