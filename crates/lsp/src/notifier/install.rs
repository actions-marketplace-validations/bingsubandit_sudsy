//! Pack mode: the "language not installed" prompt and the messages of the install command.

use super::Notifier;
use lsp_server::Request;
use lsp_server::RequestId;
use lsp_types::request::{Request as LspRequest, ShowMessageRequest};
use lsp_types::{MessageActionItem, MessageType, ShowMessageRequestParams};

/// The id of a "language not installed" prompt is this plus the language.
pub(crate) const INSTALL_PROMPT_PREFIX: &str = "sudsy/install-language/";
/// The action of that prompt, which installs the language.
pub(crate) const INSTALL_ACTION: &str = "Install";
/// The other action: the language is not offered again (per user, until it is installed).
pub(crate) const NO_PROMPT_ACTION: &str = "Don't ask again";
/// The command that installs the missing
/// languages of the open documents, or the one language given as its argument.
pub(crate) const INSTALL_COMMAND: &str = "sudsy.installMissingLanguages";

impl Notifier {
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
                "Sudsy: {language} is {why}. Install it to format and lint it. Later: run \
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
                "Sudsy: could not install {language}: {why}. Retry with \
                 `sudsy languages add {language}`."
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_server::Message;

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
