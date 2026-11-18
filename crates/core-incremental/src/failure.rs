//! Errors as query results.
//!
//! A memoized value must be comparable and cheap to share, and the error types of the layers
//! below (`ConfigError`, `ProviderError`, `FileFormatError`) are neither `Clone` nor `Eq`. A
//! query that can fail therefore returns `Result<T, Failure>`: the failure is a value that is
//! memoized, compared and backdated like any other, never a panic.
//!
//! concept: none (infrastructure: errors as query results)

use provider_api::{ProviderError, RUNTIME_MISSING, RUNTIME_URL};
use std::error::Error;
use std::fmt;

/// Which query failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// The config file does not parse or validate.
    Config,
    /// The config's format options are invalid for this file's language.
    Options,
    /// No provider handles the file. Callers ask [`crate::Database::supports`] first.
    Unsupported,
    /// The provider could not parse the file: for C# the worker is missing, crashed or refused.
    Parse,
    /// The provider's semantic model could not answer: rules that need it are skipped.
    Semantics,
    /// The runtime the C# worker needs is not installed; the message says where to get it.
    Runtime,
    /// The file has syntax errors, formatting failed, or the output failed the safety check.
    Format,
    /// A rule failed (it panicked); the file's diagnostics are not available.
    Lint,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failure {
    pub stage: Stage,
    /// The error and its causes joined with `": "`, as `anyhow`'s alternate display prints them.
    pub message: String,
}

impl Failure {
    pub(crate) fn new(stage: Stage, error: &(dyn Error + 'static)) -> Self {
        if let Some(ProviderError::RuntimeMissing(reason)) = error.downcast_ref() {
            return Failure {
                stage: Stage::Runtime,
                message: format!("{RUNTIME_MISSING} Download it from {RUNTIME_URL} ({reason})"),
            };
        }
        let mut message = error.to_string();
        let mut source = error.source();
        while let Some(cause) = source {
            message.push_str(": ");
            message.push_str(&cause.to_string());
            source = cause.source();
        }
        Failure { stage, message }
    }

    pub(crate) fn message(stage: Stage, message: impl Into<String>) -> Self {
        Failure {
            stage,
            message: message.into(),
        }
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for Failure {}

/// Runs one query's work so that a panic in a parser, formatter or rule is that file's failure
/// and not the run's end. The engine's own cancellation unwind passes through.
pub(crate) fn guarded<T>(
    stage: Stage,
    work: impl FnOnce() -> Result<T, Failure>,
) -> Result<T, Failure> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)) {
        Ok(result) => result,
        Err(payload) if payload.is::<salsa::Cancelled>() => std::panic::resume_unwind(payload),
        Err(payload) => Err(Failure::message(
            stage,
            provider_api::guard::panic_message(&*payload),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_panic_in_a_query_is_the_files_failure() {
        let failed = guarded::<()>(Stage::Lint, || panic!("boom")).unwrap_err();
        assert_eq!(failed.stage, Stage::Lint);
        assert!(failed.message.contains("boom"), "{failed}");
    }
}
