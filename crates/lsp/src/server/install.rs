//! Pack mode: the answers to the install prompt and the install command.

use super::Server;
use crate::notifier::{INSTALL_ACTION, INSTALL_PROMPT_PREFIX, NO_PROMPT_ACTION};
use anyhow::Result;
use lsp_types::ExecuteCommandParams;
use lsp_types::request::WorkspaceDiagnosticRefresh;
use std::path::Path;

impl Server {
    /// Offers to install the language of a document that was just opened, unless the project
    /// ignores it.
    pub(super) fn prompt_missing(&self, path: &Path) {
        if let Some((language, why)) = self.workspace.missing_language(path)
            && !self.workspace.ignores_language(&language)
        {
            self.notifier.language_missing(&language, &why);
        }
    }

    /// The answer to an install prompt; `None` when `id` is another request's.
    pub(super) fn install_prompt_answered(
        &mut self,
        id: &str,
        title: Option<&str>,
    ) -> Option<Result<()>> {
        let language = id.strip_prefix(INSTALL_PROMPT_PREFIX)?;
        self.notifier.prompt_answered(language);
        Some(match title {
            Some(INSTALL_ACTION) => self.install(&[language.to_string()]).map(drop),
            Some(NO_PROMPT_ACTION) => {
                // A write failure only means the language is offered again.
                let _ = core_packs::set_no_prompt(language, true);
                Ok(())
            }
            _ => Ok(()),
        })
    }

    /// Same code as `sudsy languages add`, then the documents that waited start being served.
    /// Returns what was installed and what failed (failures are also shown to the user).
    // shortcut: a download blocks the message loop for its duration, move it to a thread if
    // packs grow beyond a few hundred KB.
    fn install(&mut self, languages: &[String]) -> Result<serde_json::Value> {
        let (mut installed, mut failed) = (Vec::new(), Vec::new());
        for language in languages {
            match languages::installed::add(language, None) {
                Ok(()) => installed.push(language.clone()),
                Err(why) => {
                    self.notifier.install_failed(language, &why);
                    failed.push(serde_json::json!({ "language": language, "why": why }));
                }
            }
        }
        if !installed.is_empty() {
            self.notifier.installed(&installed);
            self.workspace.activate_installed();
            if self.support.diagnostic_refresh {
                self.call::<WorkspaceDiagnosticRefresh>(())?;
            }
        }
        Ok(serde_json::json!({ "installed": installed, "failed": failed }))
    }

    /// `sudsy.installMissingLanguages`: the language in the argument, else every missing language
    /// of the open documents.
    pub(super) fn install_missing(
        &mut self,
        params: ExecuteCommandParams,
    ) -> Result<serde_json::Value> {
        let languages = match params.arguments.first().and_then(|a| a.as_str()) {
            Some(language) => vec![language.to_string()],
            None => self.workspace.missing_open_languages(),
        };
        if languages.is_empty() {
            self.notifier.nothing_missing();
        }
        self.install(&languages)
    }
}
