//! The inputs of the database: the only values that are set from outside. Everything else is a
//! query over them.
//!
//! concept: none (infrastructure: database inputs)

use core_config::{ConfigChain, ConfigError, EditorConfig};
use std::path::PathBuf;

/// One file's text. An editor edit is `file.set_text(&mut db).to(..)` (plus a new `version`);
/// the CLI creates a file once per run.
#[salsa::input(debug)]
pub struct File {
    #[returns(ref)]
    pub path: PathBuf,
    /// The editor's document version, or 1. The worker keys its per-file state on it, so the
    /// parse query depends on it. Setting a file to the same text and a new version re-parses,
    /// which is also how a failed parse is retried.
    #[returns(copy)]
    pub version: u64,
    /// Without a leading byte-order mark.
    #[returns(ref)]
    pub text: String,
    #[returns(copy)]
    pub had_bom: bool,
    /// The `.editorconfig` values that apply to the file; the caller reads them (`EditorConfig::for_file`).
    #[returns(ref)]
    pub editorconfig: EditorConfig,
}

impl File {
    /// A file from its bytes as read from disk or sent by an editor: UTF-8 without the BOM, and
    /// the `.editorconfig` values that apply to `path`. `label` names the file in the error.
    pub fn from_bytes(
        db: &dyn crate::Db,
        path: PathBuf,
        label: impl Into<String>,
        version: u64,
        bytes: Vec<u8>,
    ) -> Result<File, provider_api::SourceError> {
        let source = provider_api::SourceFile::from_bytes(label, version, bytes)?;
        let editorconfig = EditorConfig::for_file(&path);
        Ok(File::new(
            db,
            path,
            version,
            source.text,
            source.had_bom,
            editorconfig,
        ))
    }
}

/// The config files that apply: the `sudsy.yaml` found by the caller and the files it extends, or
/// the error from reading them. An empty chain is the default config.
#[salsa::input(singleton, debug)]
pub struct ConfigFile {
    #[returns(ref)]
    pub chain: Result<ConfigChain, ConfigError>,
}

/// Bumped by the editor whenever any document opens, changes or closes. Semantic answers read it,
/// since another file's edit can change them; format and syntax-only lint do not.
#[salsa::input(singleton, debug)]
pub struct WorkspaceRevision {
    #[returns(copy)]
    pub revision: u64,
}

/// The rules that pattern files and plugins add to the providers' own (empty by default).
#[salsa::input(singleton, debug)]
pub struct ExtensionSet {
    #[returns(ref)]
    pub extensions: core_rules::Extensions,
}

/// The files of a project and how to find what they do not hold: the symbol index reads it
/// (`crate::index`). Only an index query reads it, so a run without index rules never builds one.
#[salsa::input]
pub struct Project {
    /// The project's own files.
    #[returns(ref)]
    pub files: Vec<File>,
    /// `files` is every file of the project: only then can a name that none declares be absent.
    #[returns(copy)]
    pub complete: bool,
    /// Finds the files of names and imports outside `files`; `None` when the language has no
    /// package system or no manifest was found.
    #[returns(ref)]
    pub locator: Option<std::sync::Arc<dyn core_packages::Locator>>,
    /// Bumped when a manifest or a dependency file changed on disk: files read on demand are
    /// read again.
    #[returns(copy)]
    pub revision: u64,
}

/// One project of one language: the folder its manifest is in, and its [`Project`] input.
#[derive(Clone)]
pub struct ProjectEntry {
    /// Position in the database's providers.
    pub provider: usize,
    pub root: PathBuf,
    pub project: Project,
}

/// The projects of a run or a workspace, empty until a caller that runs index rules opens them
/// (`Database::set_projects`).
#[salsa::input(singleton)]
pub struct Projects {
    #[returns(ref)]
    pub list: Vec<ProjectEntry>,
}
