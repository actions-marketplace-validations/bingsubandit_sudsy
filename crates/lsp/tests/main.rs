//! The crate's integration tests: one test binary instead of one per file.

mod common;
mod csharp;
mod directory;
#[cfg(feature = "pack-mode")]
mod languages;
mod responsiveness;
mod session;
