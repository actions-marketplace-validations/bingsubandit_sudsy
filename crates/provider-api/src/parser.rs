//! A [`LanguageProvider`] for an in-process language, configured by data: the parse function and
//! what `lang.yaml` says. Every language whose provider is generated (tree-sitter, adapters to
//! external parsers, hand-written parsers with no dialects) is one of these.

use crate::provider::{
    Capabilities, FileMatcher, FilePattern, LanguageProvider, ParseResult, ProviderError,
    SourceFile,
};
use crate::snippets::{FnSnippets, LazyRules};
use core_config::OptionSchema;
use core_format::NodeFormatter;
use core_rules::{Family, Rule, RuleHooks};
use core_syntax::grammars::LazyGrammar;
use core_syntax::{Embedded, Grammar, LanguageId, SyntaxNode};
use std::sync::Arc;

/// What `lang.yaml` and the language crate say about a language.
pub struct ParserSpec {
    /// `env!("CARGO_PKG_NAME")` of the language's crate.
    pub crate_name: &'static str,
    pub grammar: LazyGrammar,
    /// File extensions without the dot.
    pub extensions: &'static [&'static str],
    /// Exact file names (`.editorconfig`).
    pub file_names: &'static [&'static str],
    pub schema: OptionSchema,
    pub formatter: &'static dyn NodeFormatter,
    pub hooks: &'static dyn RuleHooks,
    /// The language's own Rust rules.
    pub rules: &'static [&'static dyn Rule],
    /// The text of the built-in pattern files (`pattern_files!()`).
    pub pattern_files: &'static [&'static str],
    /// The families the pattern files may name.
    pub families: &'static [Family],
}

pub struct ParserProvider {
    crate_name: &'static str,
    grammar: LazyGrammar,
    matchers: Vec<FileMatcher>,
    schema: OptionSchema,
    /// The parse function, which also parses the pattern snippets.
    parse: Arc<FnSnippets>,
    formatter: &'static dyn NodeFormatter,
    hooks: &'static dyn RuleHooks,
    rules: LazyRules,
    embedded: Option<fn(&SyntaxNode, &str) -> Vec<Embedded>>,
}

impl ParserProvider {
    /// `parse` parses a file; `SourceFile::new("pattern", 1, text)` is a pattern snippet.
    pub fn new(
        spec: ParserSpec,
        parse: impl Fn(&SourceFile) -> ParseResult + Send + Sync + 'static,
    ) -> Self {
        let parse = FnSnippets::of_files(spec.grammar, parse);
        ParserProvider {
            crate_name: spec.crate_name,
            grammar: spec.grammar,
            matchers: spec
                .extensions
                .iter()
                .map(|e| FileMatcher::extension(e))
                .chain(spec.file_names.iter().map(|n| FileMatcher {
                    pattern: FilePattern::FileName((*n).into()),
                    dialect: None,
                }))
                .collect(),
            schema: spec.schema,
            rules: LazyRules::new(
                spec.rules,
                spec.grammar.language,
                spec.pattern_files,
                spec.families,
                parse.clone(),
            ),
            parse,
            formatter: spec.formatter,
            hooks: spec.hooks,
            embedded: None,
        }
    }

    /// Finds the code of other languages in a tree (a function of the language crate).
    pub fn with_embedded(mut self, find: fn(&SyntaxNode, &str) -> Vec<Embedded>) -> Self {
        self.embedded = Some(find);
        self
    }
}

impl LanguageProvider for ParserProvider {
    fn id(&self) -> LanguageId {
        self.grammar.id
    }

    fn language(&self) -> &str {
        self.grammar.language
    }

    fn crate_name(&self) -> &'static str {
        self.crate_name
    }

    fn grammar(&self) -> &Grammar {
        self.grammar.get()
    }

    fn file_matchers(&self) -> &[FileMatcher] {
        &self.matchers
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::NONE
    }

    fn parse(&self, file: &SourceFile) -> Result<ParseResult, ProviderError> {
        Ok(self.parse.parse_file(file))
    }

    fn embedded(&self, root: &SyntaxNode, text: &str) -> Vec<Embedded> {
        self.embedded.map_or_else(Vec::new, |find| find(root, text))
    }

    fn formatter(&self) -> &dyn NodeFormatter {
        self.formatter
    }

    fn option_schema(&self) -> &OptionSchema {
        &self.schema
    }

    fn rules(&self) -> &'static [&'static dyn Rule] {
        self.rules.all()
    }

    fn rust_rules(&self) -> &'static [&'static dyn Rule] {
        self.rules.rust()
    }

    fn rule_hooks(&self) -> &dyn RuleHooks {
        self.hooks
    }
}
