//! A language that is not installed: the server offers to install it and serves the open
//! documents once the user chose the action (no download happens before).

use crate::common::{Client, apply};

fn prompts(client: &Client) -> Vec<serde_json::Value> {
    (client.server_request_params.iter())
        .filter(|(m, _)| m == "window/showMessageRequest")
        .map(|(_, p)| p.clone())
        .collect()
}

#[test]
fn a_missing_language_is_offered_once_and_served_after_the_install() {
    let t = tempfile::tempdir().unwrap();
    // The cache directory is process-wide and `set_var` is unsafe (forbidden here): run this
    // test again in a child process that has it set.
    if std::env::var_os("SUDSY_CACHE_DIR").is_none() {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "languages::a_missing_language_is_offered_once_and_served_after_the_install",
            ])
            .env("SUDSY_CACHE_DIR", t.path().join("cache"))
            .status()
            .unwrap();
        assert!(status.success());
        return;
    }
    languages::installed::remove("toml").unwrap();

    let mut client = Client::start(t.path(), languages::providers());
    let a = client.open("a.toml", "toml", "a=1\n");
    let b = client.open("b.toml", "toml", "b=1\n");
    // The harness chooses the first action; the server installs, then serves `a` and `b`.
    assert!(client.format(&a).is_empty());
    assert_eq!(apply("b=1\n", &client.format(&b)), "b = 1\n");
    assert_eq!(apply("a=1\n", &client.format(&a)), "a = 1\n");

    // One prompt for the language, though two files of it were opened.
    let shown = prompts(&client);
    assert_eq!(shown.len(), 1, "{shown:?}");
    let messages: Vec<&str> = shown
        .iter()
        .map(|p| p["message"].as_str().unwrap())
        .collect();
    assert!(
        messages.iter().any(|m| m.contains("toml is not installed")),
        "{messages:?}"
    );
    assert_eq!(shown[0]["actions"][0]["title"], "Install");
    assert!(
        client
            .server_requests
            .iter()
            .any(|m| m == "workspace/diagnostic/refresh")
    );
    assert!(languages::installed::check("toml").is_ok());

    // Dismissed prompts come back at the next open; "Don't ask again" stops them, also in a
    // new server, until the language is installed again.
    languages::installed::remove("toml").unwrap();
    client.choice = Some("dismiss");
    let before = prompts(&client).len();
    for name in ["d.toml", "e.toml"] {
        let uri = client.open(name, "toml", "d=1\n");
        client.format(&uri);
    }
    assert_eq!(prompts(&client).len(), before + 2);
    client.choice = Some("Don't ask again");
    for name in ["f.toml", "g.toml"] {
        let uri = client.open(name, "toml", "d=1\n");
        client.format(&uri);
    }
    assert_eq!(prompts(&client).len(), before + 3);
    client.choice = None;
    let mut again = Client::start(t.path(), languages::providers());
    let uri = again.open("h.toml", "toml", "d=1\n");
    again.format(&uri);
    assert!(prompts(&again).is_empty());
    assert!(core_packs::is_no_prompt("toml"));
    again.shutdown();

    // The command installs the missing languages of the open documents, or the one it is given.
    languages::installed::remove("toml").unwrap();
    let c = client.open("c.toml", "toml", "c=1\n");
    let result = client.install_command(None);
    assert_eq!(result["installed"][0], "toml", "{result}");
    assert_eq!(apply("c=1\n", &client.format(&c)), "c = 1\n");
    languages::installed::remove("toml").unwrap();
    client.close(&c);
    let result = client.install_command(Some("toml"));
    assert_eq!(result["installed"][0], "toml", "{result}");
    assert!(languages::installed::check("toml").is_ok());
    assert!(
        client.install_command(None)["installed"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
