//! Pack mode: the files of a language that is not installed are skipped with one stderr line per
//! language (`docs/ai/architecture/language-packs.md`).

use core_incremental::{Database, Db};
use languages::Project;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

/// Set when files were skipped because their language is not installed.
static SKIPPED: AtomicBool = AtomicBool::new(false);

/// Whether `skip` skipped files of a language the project does not ignore.
pub fn skipped() -> bool {
    SKIPPED.load(Ordering::Relaxed)
}

/// Drops the files of languages that are not installed and prints one stderr line per language,
/// unless the config lists it as ignored.
pub fn skip(db: &Database, project: &Project, paths: Vec<PathBuf>) -> Vec<PathBuf> {
    use core_config::Need;
    let needs = project.chain.needs();
    let mut checker = languages::installed::Checker::new(db.providers());
    let mut skipped: BTreeMap<&str, (String, usize)> = BTreeMap::new();
    let kept = paths
        .into_iter()
        .filter(|path| {
            let Some((language, why)) = checker.missing(path) else {
                return true;
            };
            if needs.get(language) != Some(&Need::Ignored) {
                skipped.entry(language).or_insert((why, 0)).1 += 1;
            }
            false
        })
        .collect();
    for (language, (why, count)) in &skipped {
        eprintln!(
            "warning: {count} {language} file(s) skipped: {why}; run `sudsy languages add {language}`"
        );
        SKIPPED.store(true, Ordering::Relaxed);
    }
    kept
}
