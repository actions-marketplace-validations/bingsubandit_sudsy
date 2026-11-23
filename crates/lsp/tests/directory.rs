//! A module's rules in the editor: the findings of a file follow its sibling files, the open
//! documents (unsaved) and the disk's.

use crate::common::Client;
use lsp_types::NumberOrString;

fn rules(client: &mut Client, uri: &lsp_types::Uri) -> Vec<String> {
    client
        .diagnostics(uri)
        .iter()
        .filter_map(|d| match d.code.as_ref()? {
            NumberOrString::String(s) if s == "unused-declarations" => Some(s.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn findings_follow_unsaved_documents_and_the_disk() {
    let t = tempfile::tempdir().unwrap();
    std::fs::write(
        t.path().join("sudsy.yaml"),
        "lint:\n  rules:\n    unused-declarations: warn\n",
    )
    .unwrap();
    let mut client = Client::start(t.path(), languages::providers());
    let main = client.open("main.tf", "terraform", "variable \"a\" {}\n");
    assert_eq!(rules(&mut client, &main).len(), 1);

    // A new file on disk that uses the variable.
    std::fs::write(t.path().join("use.tf"), "output \"o\" { value = var.a }\n").unwrap();
    assert!(rules(&mut client, &main).is_empty());

    // The same file open with an unsaved edit that no longer does.
    let user = client.open("use.tf", "terraform", "output \"o\" { value = var.b }\n");
    assert_eq!(rules(&mut client, &main).len(), 1);
    client.change(&user, 2, None, "output \"o\" { value = var.a }\n");
    assert!(rules(&mut client, &main).is_empty());
    client.close(&user);

    // Deleted from disk.
    std::fs::remove_file(t.path().join("use.tf")).unwrap();
    assert_eq!(rules(&mut client, &main).len(), 1);
    client.shutdown();
}
