//! Messages to the user (`window/showMessage`, `window/logMessage`), shared by the message loop
//! and the analysis thread so that one broken config is one message, whoever notices it.

use core_incremental::{Failure, Stage};
use crossbeam_channel::Sender;
use lsp_server::{Message, Notification, Request, RequestId};
use lsp_types::notification::{LogMessage, Notification as LspNotification, ShowMessage};
use lsp_types::request::{Request as LspRequest, ShowMessageRequest};
use lsp_types::{
    LogMessageParams, MessageActionItem, MessageType, ShowMessageParams, ShowMessageRequestParams,
    Uri,
};
use provider_api::RUNTIME_MISSING;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// The id of the "runtime missing" prompt, to recognize its answer.
pub(crate) const RUNTIME_PROMPT_ID: &str = "sudsy/runtime-missing";
/// The prompt's action, which opens the download page.
pub(crate) const DOWNLOAD_ACTION: &str = "Download .NET";
/// The id of a "language not installed" prompt is this plus the language.
pub(crate) const INSTALL_PROMPT_PREFIX: &str = "sudsy/install-language/";
/// The action of that prompt, which installs the language.
pub(crate) const INSTALL_ACTION: &str = "Install";
/// The other action: the language is not offered again (per user, until it is installed).
pub(crate) const NO_PROMPT_ACTION: &str = "Don't ask again";
/// The command (palette entry "Sudsy: Install Missing Languages") that installs the missing
/// languages of the open documents, or the one language given as its argument.
pub(crate) const INSTALL_COMMAND: &str = "sudsy.installMissingLanguages";

#[derive(Clone)]
pub(crate) struct Notifier {
    sender: Sender<Message>,
    /// Failures already shown to the user.
    shown: Arc<Mutex<HashSet<String>>>,
    /// Has the missing runtime been put to the user? Once per session, also across config reloads.
    runtime_prompted: Arc<AtomicBool>,
    /// Languages whose install prompt is still unanswered.
    install_pending: Arc<Mutex<HashSet<String>>>,
}

impl Notifier {
    pub(crate) fn new(sender: Sender<Message>) -> Self {
        Notifier {
            sender,
            shown: Default::default(),
            runtime_prompted: Default::default(),
            install_pending: Default::default(),
        }
    }

    /// Forgets what was shown, so that a reloaded config reports its problems again.
    pub(crate) fn reset(&self) {
        self.shown.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }

    fn first_time(&self, message: &str) -> bool {
        self.shown
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(message.to_string())
    }

    fn send<N: LspNotification>(&self, params: N::Params)
    where
        N::Params: serde::Serialize,
    {
        let _ = self
            .sender
            .send(Notification::new(N::METHOD.to_string(), params).into());
    }

    fn show(&self, typ: MessageType, message: String) {
        self.send::<ShowMessage>(ShowMessageParams { typ, message });
    }

    fn log(&self, typ: MessageType, message: String) {
        self.send::<LogMessage>(LogMessageParams { typ, message });
    }

    fn prompt_runtime_missing(&self) {
        if self.runtime_prompted.swap(true, Ordering::Relaxed) {
            return;
        }
        let params = ShowMessageRequestParams {
            typ: MessageType::WARNING,
            message: RUNTIME_MISSING.into(),
            actions: Some(vec![MessageActionItem {
                title: DOWNLOAD_ACTION.into(),
                properties: Default::default(),
            }]),
        };
        let request = Request::new(
            RequestId::from(RUNTIME_PROMPT_ID.to_string()),
            ShowMessageRequest::METHOD.to_string(),
            params,
        );
        let _ = self.sender.send(request.into());
    }

    /// A language the user needs is not installed: offers to install it every time a file of it
    /// opens, unless the user chose "Don't ask again" or the last offer is still unanswered.
    /// Nothing is downloaded before the user chooses the action.
    pub(crate) fn language_missing(&self, language: &str, why: &str) {
        if core_packs::is_no_prompt(language) {
            return;
        }
        let first = (self.install_pending.lock())
            .unwrap_or_else(|e| e.into_inner())
            .insert(language.to_string());
        if !first {
            return;
        }
        let params = ShowMessageRequestParams {
            typ: MessageType::WARNING,
            message: format!(
                "Sudsy: {language} is {why}. Install it to format and lint it. Later: run \"Sudsy: \
                 Install Missing Languages\" in the command palette, or \
                 `sudsy languages add {language}`."
            ),
            actions: Some(
                [INSTALL_ACTION, NO_PROMPT_ACTION]
                    .map(|title| MessageActionItem {
                        title: title.into(),
                        properties: Default::default(),
                    })
                    .into(),
            ),
        };
        let id = RequestId::from(format!("{INSTALL_PROMPT_PREFIX}{language}"));
        let request = Request::new(id, ShowMessageRequest::METHOD.to_string(), params);
        let _ = self.sender.send(request.into());
    }

    /// The prompt for `language` was answered (or dismissed).
    pub(crate) fn prompt_answered(&self, language: &str) {
        (self.install_pending.lock())
            .unwrap_or_else(|e| e.into_inner())
            .remove(language);
    }

    pub(crate) fn install_failed(&self, language: &str, why: &str) {
        self.show(
            MessageType::ERROR,
            format!(
                "Sudsy: could not install {language}: {why}. Retry with \"Sudsy: Install Missing \
                 Languages\" in the command palette, or `sudsy languages add {language}`."
            ),
        );
    }

    pub(crate) fn nothing_missing(&self) {
        self.show(
            MessageType::INFO,
            "Sudsy: no language of the open documents is missing".into(),
        );
    }

    pub(crate) fn installed(&self, languages: &[String]) {
        self.show(
            MessageType::INFO,
            format!("Sudsy: installed {}", languages.join(", ")),
        );
    }

    /// The config could not be loaded. The message is the one every file's failure carries, so it
    /// counts as shown for them.
    pub(crate) fn config_failed(&self, error: String) {
        let message = format!("Sudsy: {error}");
        if self.first_time(&message) {
            self.show(MessageType::ERROR, message);
        }
    }

    /// Tells the user why a file could not be processed: once per distinct reason when it is
    /// something they can fix (config, worker), in the log when it is about one file's content.
    pub(crate) fn failed(&self, uri: &Uri, failure: &Failure) {
        let message = format!("Sudsy: {}", failure.message);
        match failure.stage {
            Stage::Unsupported => {}
            Stage::Runtime => {
                self.log(
                    MessageType::WARNING,
                    format!("{}: {}", uri.as_str(), failure.message),
                );
                self.prompt_runtime_missing();
            }
            Stage::Format | Stage::Lint => self.log(
                MessageType::WARNING,
                format!("{}: {}", uri.as_str(), failure.message),
            ),
            Stage::Config | Stage::Options | Stage::Parse | Stage::Semantics => {
                if self.first_time(&message) {
                    let level = if matches!(failure.stage, Stage::Parse | Stage::Semantics) {
                        MessageType::WARNING
                    } else {
                        MessageType::ERROR
                    };
                    self.show(level, message);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_unanswered_prompt_per_language_and_the_failure_names_the_cli() {
        let (sender, receiver) = crossbeam_channel::unbounded();
        let notifier = Notifier::new(sender);
        notifier.language_missing("toml", "not installed");
        notifier.language_missing("toml", "not installed");
        assert_eq!(receiver.try_iter().count(), 1);
        notifier.prompt_answered("toml");
        notifier.language_missing("toml", "not installed");
        assert_eq!(receiver.try_iter().count(), 1);
        notifier.install_failed("toml", "offline");
        let sent: Vec<Message> = receiver.try_iter().collect();
        assert!(matches!(&sent[..], [Message::Notification(n)]
            if n.params["message"].as_str().unwrap().contains("sudsy languages add toml")));
    }
}
