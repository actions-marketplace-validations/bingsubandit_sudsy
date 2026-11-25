//! The rules that read the symbol index in the editor: the projects open when such a rule is
//! active, follow the documents and the manifests, and stay closed otherwise.

use crate::common::Client;
use lang_protobuf::provider;
use lsp_types::{NumberOrString, Uri};
use provider_api::LanguageProvider;

const ON: &str = "lint:\n  rules:\n    import-used: warn\n";
const HEAD: &str = "syntax = \"proto3\";\npackage p;\n";

fn unused(client: &mut Client, uri: &Uri) -> usize {
    client
        .diagnostics(uri)
        .iter()
        .filter(|d| matches!(&d.code, Some(NumberOrString::String(s)) if s == "import-used"))
        .count()
}

fn start(root: &std::path::Path) -> Client {
    let providers: Vec<Box<dyn LanguageProvider>> = vec![Box::new(provider())];
    Client::start(root, providers)
}

#[test]
fn an_unused_import_is_reported_once_a_manifest_exists_and_follows_edits() {
    let t = tempfile::tempdir().unwrap();
    std::fs::write(t.path().join("sudsy.yaml"), ON).unwrap();
    std::fs::write(t.path().join("c.proto"), format!("{HEAD}message C {{}}\n")).unwrap();
    let mut client = start(t.path());
    let a = client.open(
        "a.proto",
        "proto",
        &format!("{HEAD}import \"c.proto\";\nmessage A {{}}\n"),
    );
    assert_eq!(unused(&mut client, &a), 0, "no buf.yaml: no project");

    std::fs::write(t.path().join("buf.yaml"), "version: v2\n").unwrap();
    client.file_changed("buf.yaml");
    assert_eq!(unused(&mut client, &a), 1, "the manifest appeared");

    client.change(
        &a,
        2,
        None,
        &format!("{HEAD}import \"c.proto\";\nmessage A {{ C c = 1; }}\n"),
    );
    assert_eq!(unused(&mut client, &a), 0, "the edit uses it");
    client.shutdown();
}

#[test]
fn with_no_index_rule_active_no_project_is_opened() {
    let t = tempfile::tempdir().unwrap();
    std::fs::write(t.path().join("buf.yaml"), "version: v2\n").unwrap();
    let providers: Vec<Box<dyn LanguageProvider>> = vec![Box::new(provider())];
    let (mut off, _) = lsp::Workspace::new(t.path().to_path_buf(), providers);
    off.open(
        t.path().join("a.proto"),
        1,
        format!("{HEAD}message A {{}}\n"),
    );
    off.refresh_index();
    assert!(!off.index_open());

    std::fs::write(t.path().join("sudsy.yaml"), ON).unwrap();
    off.reload_config();
    off.refresh_index();
    assert!(
        off.index_open(),
        "a rule that reads the index is active now"
    );
}

#[test]
fn a_source_file_changed_on_disk_while_closed_is_read_again() {
    let t = tempfile::tempdir().unwrap();
    std::fs::write(t.path().join("sudsy.yaml"), ON).unwrap();
    std::fs::write(t.path().join("buf.yaml"), "version: v2\n").unwrap();
    std::fs::write(t.path().join("c.proto"), format!("{HEAD}message C {{}}\n")).unwrap();
    let mut client = start(t.path());
    let a = client.open(
        "a.proto",
        "proto",
        &format!("{HEAD}import \"c.proto\";\nmessage A {{ C c = 1; }}\n"),
    );
    assert_eq!(unused(&mut client, &a), 0);
    std::fs::write(t.path().join("c.proto"), format!("{HEAD}message D {{}}\n")).unwrap();
    client.file_changed("c.proto");
    assert_eq!(unused(&mut client, &a), 1, "c.proto no longer declares C");
    client.shutdown();
}
