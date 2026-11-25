//! Every release links all languages in: no install state hides one, and `sudsy languages` does
//! not exist. Pack mode has its own tests (`languages.rs`).

use std::fs;
use std::process::{Command, Output};

fn sudsy(dir: &std::path::Path, cache: &std::path::Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sudsy"))
        .args(args)
        .current_dir(dir)
        .env("SUDSY_CACHE_DIR", cache)
        .output()
        .unwrap()
}

#[test]
fn leftover_install_state_does_not_hide_a_built_in_language() {
    let t = tempfile::tempdir().unwrap();
    let (dir, cache) = (t.path().join("proj"), t.path().join("cache"));
    fs::create_dir_all(&dir).unwrap();
    fs::create_dir_all(&cache).unwrap();
    fs::write(dir.join("a.toml"), "a   = 1\n").unwrap();
    fs::write(dir.join("Dockerfile"), "FROM  alpine\n").unwrap();
    // The state of an older version that removed languages, and a pack that is not one.
    fs::write(
        cache.join("state.json"),
        r#"{"removed": ["toml", "dockerfile"], "no_prompt": ["toml"]}"#,
    )
    .unwrap();
    fs::write(
        cache.join(format!("sudsy-{}-toml-x.so", env!("CARGO_PKG_VERSION"))),
        "not a library",
    )
    .unwrap();
    // `required` and `ignored` are still accepted.
    fs::write(
        dir.join("sudsy.yaml"),
        "languages: { toml: required, ini: ignored }\n",
    )
    .unwrap();

    let out = sudsy(&dir, &cache, &["format"]);
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{err}");
    assert!(!err.contains("skipped"), "{err}");
    assert_eq!(fs::read_to_string(dir.join("a.toml")).unwrap(), "a = 1\n");
    assert_eq!(
        fs::read_to_string(dir.join("Dockerfile")).unwrap(),
        "FROM alpine\n"
    );
    assert_eq!(sudsy(&dir, &cache, &["lint"]).status.code(), Some(0));
}

#[test]
fn there_is_no_languages_command() {
    let t = tempfile::tempdir().unwrap();
    let out = sudsy(t.path(), t.path(), &["languages", "list"]);
    assert_eq!(out.status.code(), Some(2));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("unrecognized subcommand 'languages'"), "{err}");
    let help = sudsy(t.path(), t.path(), &["--help"]);
    assert!(!String::from_utf8_lossy(&help.stdout).contains("anguages"));
}
