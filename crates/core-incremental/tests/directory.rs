//! Directory scope: the findings of a module's rules follow its files, the open ones and the
//! disk's, and never invent an absence the files do not prove.

use core_config::{ConfigChain, EditorConfig};
use core_incremental::{Database, File, Siblings, directory_scope, module_diagnostics};
use provider_api::LanguageProvider;
use std::path::{Path, PathBuf};

const CONFIG: &str =
    "lint:\n  rules:\n    unused-declarations: warn\n    required-providers: warn\n";

fn db(config: &str) -> Database {
    let providers: Vec<Box<dyn LanguageProvider>> = vec![Box::new(lang_hcl::provider())];
    let mut db = Database::new(providers);
    db.set_config(ConfigChain::parse(config));
    db
}

fn write(dir: &Path, name: &str, text: &str) {
    std::fs::write(dir.join(name), text).unwrap();
}

/// `(file name, message)` of the unit's findings for the directory, with `open` documents
/// standing for their files.
fn findings(
    db: &Database,
    siblings: &Siblings,
    dir: &Path,
    open: &[File],
) -> Vec<(String, String)> {
    let main = dir.join("main.tf");
    let file = open
        .iter()
        .copied()
        .find(|f| f.path(db) == &main)
        .unwrap_or_else(|| {
            File::new(
                db,
                main.clone(),
                1,
                String::new(),
                false,
                EditorConfig::default(),
            )
        });
    let scope = directory_scope(db, file).expect("the rules are on");
    let unit = siblings.unit(db, dir, &scope, open);
    let mut found: Vec<_> = module_diagnostics(db, unit)
        .as_ref()
        .unwrap()
        .iter()
        .map(|(p, d)| {
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                d.message.clone(),
            )
        })
        .collect();
    found.sort();
    found
}

fn open(db: &Database, dir: &Path, name: &str, text: &str) -> File {
    File::new(
        db,
        dir.join(name),
        1,
        text.into(),
        false,
        EditorConfig::default(),
    )
}

#[test]
fn a_sibling_that_is_edited_added_or_deleted_changes_the_findings() {
    let t = tempfile::tempdir().unwrap();
    let db = db(CONFIG);
    let siblings = Siblings::default();
    write(t.path(), "main.tf", "variable \"a\" {}\n");
    assert_eq!(findings(&db, &siblings, t.path(), &[]).len(), 1);

    // Added: another file uses the variable.
    write(t.path(), "use.tf", "output \"o\" { value = var.a }\n");
    assert_eq!(findings(&db, &siblings, t.path(), &[]), []);
    // Edited on disk (a different size, so the cache sees it).
    write(
        t.path(),
        "use.tf",
        "output \"o\" { value = var.other_name }\n",
    );
    assert_eq!(findings(&db, &siblings, t.path(), &[]).len(), 1);
    // An open document wins over the disk.
    let doc = open(&db, t.path(), "use.tf", "output \"o\" { value = var.a }\n");
    assert_eq!(findings(&db, &siblings, t.path(), &[doc]), []);
    // Deleted.
    std::fs::remove_file(t.path().join("use.tf")).unwrap();
    assert_eq!(findings(&db, &siblings, t.path(), &[]).len(), 1);
}

#[test]
fn an_unsaved_edit_of_the_open_file_changes_the_findings() {
    let t = tempfile::tempdir().unwrap();
    let mut db = db(CONFIG);
    let siblings = Siblings::default();
    write(t.path(), "use.tf", "output \"o\" { value = var.a }\n");
    let main = open(&db, t.path(), "main.tf", "variable \"a\" {}\n");
    assert_eq!(findings(&db, &siblings, t.path(), &[main]), []);
    db.set_file_text(main, 2, "variable \"b\" {}\n".into());
    assert_eq!(findings(&db, &siblings, t.path(), &[main]).len(), 1);
}

#[test]
fn an_absence_is_not_reported_when_a_file_may_hide_it() {
    let t = tempfile::tempdir().unwrap();
    let db = db(CONFIG);
    let siblings = Siblings::default();
    write(t.path(), "main.tf", "variable \"a\" {}\n");
    assert_eq!(findings(&db, &siblings, t.path(), &[]).len(), 1);
    // A file that does not parse.
    write(t.path(), "broken.tf", "variable {{{\n");
    assert_eq!(findings(&db, &siblings, t.path(), &[]), []);
    std::fs::remove_file(t.path().join("broken.tf")).unwrap();
    // A JSON module file is not read.
    write(t.path(), "x.tf.json", "{}");
    assert_eq!(findings(&db, &siblings, t.path(), &[]), []);
    std::fs::remove_file(t.path().join("x.tf.json")).unwrap();
    assert_eq!(findings(&db, &siblings, t.path(), &[]).len(), 1);
}

#[test]
fn the_files_of_other_folders_and_other_kinds_are_not_the_module() {
    let t = tempfile::tempdir().unwrap();
    let db = db(CONFIG);
    let siblings = Siblings::default();
    write(t.path(), "main.tf", "variable \"a\" {}\n");
    std::fs::create_dir(t.path().join("sub")).unwrap();
    write(
        &t.path().join("sub"),
        "use.tf",
        "output \"o\" { value = var.a }\n",
    );
    write(t.path(), "vars.tfvars", "a = 1\n");
    assert_eq!(findings(&db, &siblings, t.path(), &[]).len(), 1);
}

#[test]
fn with_no_directory_rule_on_the_file_has_no_scope() {
    let db = db("lint:\n  rules:\n    deprecated-index: warn\n");
    let file = File::new(
        &db,
        PathBuf::from("/x/main.tf"),
        1,
        String::new(),
        false,
        EditorConfig::default(),
    );
    assert!(directory_scope(&db, file).is_none());
}
