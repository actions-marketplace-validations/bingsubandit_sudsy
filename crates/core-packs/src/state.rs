//! The per-user directory holding packs and `state.json` (the removed languages, the languages not to ask about).
//!
//! concept: none (infrastructure: language packs)

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::PathBuf;

#[derive(Default, Serialize, Deserialize)]
struct State {
    removed: BTreeSet<String>,
    /// Languages whose install prompt the user dismissed for good.
    #[serde(default)]
    no_prompt: BTreeSet<String>,
}

/// `$SUDSY_CACHE_DIR`, else `<platform cache dir>/sudsy`.
pub fn cache_dir() -> PathBuf {
    match std::env::var_os("SUDSY_CACHE_DIR") {
        Some(dir) => dir.into(),
        None => dirs::cache_dir().unwrap_or_default().join("sudsy"),
    }
}

fn read() -> State {
    std::fs::read(cache_dir().join("state.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn is_removed(language: &str) -> bool {
    read().removed.contains(language)
}

pub fn set_removed(language: &str, removed: bool) -> std::io::Result<()> {
    update(|s| flip(&mut s.removed, language, removed))
}

pub fn is_no_prompt(language: &str) -> bool {
    read().no_prompt.contains(language)
}

pub fn set_no_prompt(language: &str, no_prompt: bool) -> std::io::Result<()> {
    update(|s| flip(&mut s.no_prompt, language, no_prompt))
}

fn flip(set: &mut BTreeSet<String>, language: &str, on: bool) {
    if on {
        set.insert(language.into());
    } else {
        set.remove(language);
    }
}

fn update(change: impl FnOnce(&mut State)) -> std::io::Result<()> {
    let mut state = read();
    change(&mut state);
    std::fs::create_dir_all(cache_dir())?;
    std::fs::write(
        cache_dir().join("state.json"),
        serde_json::to_vec(&state).expect("state serializes"),
    )
}
