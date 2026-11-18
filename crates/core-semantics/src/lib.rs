//! Capability layers, symbol ids, the facts a provider's semantic model answers about a file, and
//! behavior summaries. Language neutral: a provider fills these in,
//! rules read them.
//!
//! concept: none (glue: crate root)

mod facts;
mod layer;
pub mod scopes;
mod summaries;

pub use facts::{
    Accessibility, Answers, CompilerDiagnostic, Declaration, DeclarationFlags, DeclarationKind,
    FileSemantics, Finding, Reference, Span, qualified_name,
};
pub use layer::{Layer, Layers};
pub use summaries::{BehaviorSummaries, BodyFlow, SemanticView};
