//! `cargo run --release -p core-incremental --example module -- DIR [FILES]`: what an editor pull costs in a large Terraform module. Writes
//! FILES (default 500) `.tf` files below DIR, turns the three module rules on, and times the
//! directory phase of one open file as the server runs it: the first pull (every file read and
//! its facts collected), a pull with nothing changed (readdir, one stat per file, the cached
//! query), a pull after an unsaved edit of the open file, a pull after an edit on disk, and a
//! pull with the module rules off (no directory is read).

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
use core_config::{ConfigChain, EditorConfig};
use core_incremental::{Database, File, Siblings, directory_scope, module_diagnostics};
use std::path::Path;
use std::time::Instant;

fn pull(db: &Database, siblings: &Siblings, dir: &Path, open: File) -> usize {
    let Some(scope) = directory_scope(db, open) else {
        return 0;
    };
    let unit = siblings.unit(db, dir, &scope, &[open]);
    module_diagnostics(db, unit).as_ref().map_or(0, Vec::len)
}

fn timed(label: &str, work: impl FnOnce() -> usize) {
    let started = Instant::now();
    let findings = work();
    println!(
        "{label:<28} {:>9.2} ms   {findings} findings",
        started.elapsed().as_secs_f64() * 1e3
    );
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dir = Path::new(args.first().map_or("module-bench", String::as_str));
    let files: usize = args.get(1).map_or(Ok(500), |n| n.parse())?;
    std::fs::create_dir_all(dir)?;
    for i in 0..files {
        let text = format!(
            "variable \"v{i}\" {{\n  type = string\n}}\n\nlocals {{\n  l{i} = var.v{i}\n}}\n\nresource \"aws_instance\" \"r{i}\" {{\n  ami = local.l{i}\n}}\n\noutput \"o{i}\" {{\n  value = aws_instance.r{i}.id\n}}\n"
        );
        std::fs::write(dir.join(format!("f{i:04}.tf")), text)?;
    }
    let main = dir.join("f0000.tf");
    let text = std::fs::read_to_string(&main)?;
    let rules = "lint:\n  rules:\n    unused-declarations: warn\n    required-providers: warn\n    required-version: warn\n";
    for (label, config) in [("rules off", "lint:\n  rules: {}\n"), ("rules on", rules)] {
        let mut db = Database::new(vec![Box::new(lang_hcl::provider())]);
        db.set_config(ConfigChain::parse(config));
        let open = File::new(
            &db,
            main.clone(),
            1,
            text.clone(),
            false,
            EditorConfig::default(),
        );
        let siblings = Siblings::default();
        println!("{label}, {files} files");
        timed("first pull", || pull(&db, &siblings, dir, open));
        timed("pull, nothing changed", || pull(&db, &siblings, dir, open));
        db.set_file_text(open, 2, format!("{text}# edit\n"));
        timed("pull after an unsaved edit", || {
            pull(&db, &siblings, dir, open)
        });
        std::fs::write(dir.join("f0001.tf"), "variable \"changed\" {}\n")?;
        timed("pull after a disk edit", || pull(&db, &siblings, dir, open));
    }
    std::fs::remove_dir_all(dir)?;
    Ok(())
}
