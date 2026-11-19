//! The generated part of the README "Beyond parity" section: the languages where Sudsy has more format
//! options than the reference tool (`tool-options.yaml`). Every number comes from a file of the repository.

use anyhow::Result;
use provider_api::LanguageProvider;
use serde::Deserialize;

#[derive(Deserialize)]
struct Options {
    language: String,
    name: String,
    tool: String,
    options: usize,
    source: String,
}

/// The block text: the options table.
pub fn render(providers: &[Box<dyn LanguageProvider>], options_yaml: &str) -> Result<String> {
    let options: Vec<Options> = serde_saphyr::from_str(options_yaml)?;
    Ok(options_table(providers, &options))
}

/// The languages where Sudsy declares more format options than the reference tool has.
fn options_table(providers: &[Box<dyn LanguageProvider>], options: &[Options]) -> String {
    let mut out = String::from(
        "Sudsy format options vs other tools:\n\n| Language | Sudsy options | Reference tool | Tool options |\n|---|---:|---|---:|\n",
    );
    for o in options {
        let sudsy = providers
            .iter()
            .find(|p| p.language() == o.language)
            .map_or(0, |p| p.option_schema().defs().len());
        if sudsy > o.options {
            out += &format!(
                "| {} | {sudsy} | {} | [{}]({}) |\n",
                o.name, o.tool, o.options, o.source
            );
        }
    }
    out
}
