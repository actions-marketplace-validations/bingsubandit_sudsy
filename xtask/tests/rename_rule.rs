use std::fs;
use std::path::Path;
use std::process::Command;

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

#[test]
fn renames_tokens_statuses_and_files_but_not_upstream_keys() {
    let root = std::env::temp_dir().join(format!("sudsy-rename-rule-{}", std::process::id()));
    write(
        &root,
        "crates/lang-js/rules/no-x.yaml",
        "id: no-x\nmessage: no-x-more\n",
    );
    write(&root, "tests/rules/js/no-x/a.js", "x // ^ no-x\n");
    write(
        &root,
        "docs/ai/parity/eslint.yaml",
        "entries:\n  no-x: rule no-x, other\n",
    );
    write(&root, "crates/cli/src/init/eslint.rs", "\"no-x\"\n");
    write(&root, "CHANGELOG.md", "no-x\n");
    for args in [["init", "-q"], ["add", "."]] {
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(&root)
                .status()
                .unwrap()
                .success()
        );
    }

    let said = xtask::rename_rule(&root, "no-x", "no-y", false)
        .unwrap()
        .join("\n");
    assert!(
        said.contains("crates/cli/src/init/eslint.rs: 1 hit(s) NOT rewritten"),
        "{said}"
    );
    let read = |rel: &str| fs::read_to_string(root.join(rel)).unwrap();
    assert_eq!(
        read("crates/lang-js/rules/no-y.yaml"),
        "id: no-y\nmessage: no-x-more\n"
    );
    assert_eq!(read("tests/rules/js/no-y/a.js"), "x // ^ no-y\n");
    assert_eq!(
        read("docs/ai/parity/eslint.yaml"),
        "entries:\n  no-x: rule no-y, other\n"
    );
    assert_eq!(read("CHANGELOG.md"), "no-x\n");
    fs::remove_dir_all(&root).unwrap();
}
