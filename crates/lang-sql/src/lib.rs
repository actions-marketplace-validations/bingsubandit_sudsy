//! The SQL provider (`.sql`): sqruff's parser (ansi, postgres, mysql and tsql dialects) behind
//! `adapter`, which builds the lossless tree; the formatter prints the file as written.

core_syntax::include_generated!();
mod adapter;
mod format;
mod rules;

pub use adapter::{DIALECTS, MAX_BYTES, parse};
pub use format::{DIALECT, SqlFormatter, option_schema};

use core_config::OptionSchema;
use core_format::NodeFormatter;
use core_rules::Rule;
use core_syntax::{Grammar, LanguageId, grammars};
use provider_api::{
    Capabilities, FileMatcher, FnSnippets, LanguageProvider, LazyRules, ParseResult, ProviderError,
    SourceFile,
};

pub struct SqlProvider {
    matchers: Vec<FileMatcher>,
    schema: OptionSchema,
    formatter: SqlFormatter,
    rules: LazyRules,
}

impl Default for SqlProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl SqlProvider {
    pub fn new() -> Self {
        SqlProvider {
            matchers: vec![FileMatcher::extension("sql")],
            schema: option_schema(),
            formatter: SqlFormatter,
            rules: LazyRules::new(
                rules::RULES,
                grammars::SQL.language,
                provider_api::pattern_files!(),
                &[],
                FnSnippets::new(grammars::SQL, |text| parse(text, "ansi")),
            ),
        }
    }
}

impl LanguageProvider for SqlProvider {
    fn id(&self) -> LanguageId {
        grammars::SQL.id
    }

    fn language(&self) -> &str {
        grammars::SQL.language
    }

    fn crate_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }

    fn grammar(&self) -> &Grammar {
        grammars::SQL.get()
    }

    fn file_matchers(&self) -> &[FileMatcher] {
        &self.matchers
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::NONE
    }

    fn parse(&self, file: &SourceFile) -> Result<ParseResult, ProviderError> {
        let dialect = file.language_version.as_deref().unwrap_or("ansi");
        Ok(parse(&file.text, dialect))
    }

    fn formatter(&self) -> &dyn NodeFormatter {
        &self.formatter
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
}
