//! The generated part of the README "Highly configurable" section: the format options of Sudsy against those of
//! every reference formatter of `support.yaml` (`tool-options.yaml`). Every number comes from a file of the repository.

use anyhow::{Result, bail};
use core_config::OptionSchema;
use provider_api::LanguageProvider;
use serde::Deserialize;

#[derive(Deserialize)]
struct Options {
    /// The first language id of the `support.yaml` row.
    language: String,
    /// The name of the format tool in that row.
    tool: String,
    options: usize,
    source: String,
    /// How the number was counted (read by people, not by the table).
    #[allow(dead_code)]
    note: String,
}

/// The block text: the options table.
pub fn render(
    providers: &[Box<dyn LanguageProvider>],
    options_yaml: &str,
    support_yaml: &str,
) -> Result<String> {
    let options: Vec<Options> = serde_saphyr::from_str(options_yaml)?;
    let rows = crate::support::rows(support_yaml)?;
    options_table(
        &|id| {
            let rules = core_rules::shared::RULES
                .iter()
                .filter(|r| r.formats())
                .count();
            providers
                .iter()
                .find(|p| p.language() == id)
                .map_or(0, |p| sudsy_options(p.option_schema(), rules))
        },
        &rows,
        &options,
    )
}

/// The Sudsy count: every option of the schema; an option that switches rules by id (`format_rules`)
/// counts one per rule it can name (`rules`), as the rule switches of a tool do.
fn sudsy_options(schema: &OptionSchema, rules: usize) -> usize {
    let switch = core_config::opt::FORMAT_RULES.name();
    let defs = schema.defs();
    if defs.iter().any(|d| d.name == switch) {
        defs.len() - 1 + rules
    } else {
        defs.len()
    }
}

/// One row per reference formatter of the support table, in its order, with the option count of the
/// first language id of its row. A formatter without a row of `tool-options.yaml`, and a row without a
/// formatter, are errors.
fn options_table(
    sudsy: &dyn Fn(&str) -> usize,
    support: &[crate::support::Row],
    options: &[Options],
) -> Result<String> {
    let mut out = String::from(
        "**Extensive formatting and lint rules.** Sudsy format options vs other tools:\n\n| Language | Sudsy options | Reference tool | Tool options |\n|---|---:|---|---:|\n",
    );
    let mut used = 0;
    for row in support {
        let id = row.ids.first().map_or("", String::as_str);
        for tool in &row.format {
            let Some(o) = options
                .iter()
                .find(|o| o.language == id && o.tool == tool.name)
            else {
                bail!(
                    "tool-options.yaml has no row for the format tool {} of {} ({id})",
                    tool.name,
                    row.language
                );
            };
            used += 1;
            out += &format!(
                "| {} | {} | {} | [{}]({}) |\n",
                row.language,
                sudsy(id),
                o.tool,
                o.options,
                o.source
            );
        }
    }
    if used != options.len() {
        bail!("tool-options.yaml has rows that match no format tool of support.yaml");
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SUPPORT: &str = "- {language: Foo, ids: [foo, bar], format: [{tool: a, name: A}, {tool: b, name: B}]}\n- {language: Lint, ids: [lint], lint: [{tool: l, name: L}]}\n";

    fn opt(tool: &str, n: usize) -> String {
        format!("- {{language: foo, tool: {tool}, options: {n}, source: \"https://x\", note: n}}\n")
    }

    fn table(options: &str) -> Result<String> {
        let rows = crate::support::rows(SUPPORT)?;
        let options: Vec<Options> = serde_saphyr::from_str(options)?;
        options_table(&|_| 3, &rows, &options)
    }

    #[test]
    fn a_rule_list_option_counts_one_setting_per_rule() {
        let list = core_config::shared::format_rules(&[]);
        let flag = core_config::OptionDef::boolean("final_newline", true, "x");
        assert_eq!(sudsy_options(&OptionSchema::new(vec![flag.clone()]), 4), 1);
        assert_eq!(sudsy_options(&OptionSchema::new(vec![flag, list]), 4), 5);
    }

    #[test]
    fn a_tool_with_more_options_than_sudsy_keeps_its_row() {
        let t = table(&(opt("A", 40) + &opt("B", 1))).unwrap();
        assert!(
            t.contains("| Foo | 3 | A | [40](https://x) |\n| Foo | 3 | B | [1](https://x) |\n")
        );
    }

    #[test]
    fn a_format_tool_without_a_row_fails() {
        let e = table(&opt("A", 1)).unwrap_err().to_string();
        assert!(e.contains("no row for the format tool B"), "{e}");
    }

    #[test]
    fn a_row_without_a_format_tool_fails() {
        assert!(table(&(opt("A", 1) + &opt("B", 1) + &opt("C", 1))).is_err());
    }
}
