//! `docs/languages.md`: its language sections in the order of every language list (`support::ordered`), and in each
//! section a generated block `measured-<heading in lowercase words joined by ->` with the numbers of the row from the parity
//! ratchets, so that no measured number is typed in the page. A row without a `## <section>` heading, a language
//! section apart from the others, or a section without a block fails the build with its name.

use crate::docs::splice_text;
use crate::support::{Measured, Tool};
use anyhow::{Result, bail};
use std::path::PathBuf;

const PATH: &str = "docs/languages.md";

/// The page with its language sections ordered and the measured blocks filled.
pub(crate) fn generate(
    text: &str,
    rows: &[Measured],
    rule_count: &dyn Fn(&[String]) -> usize,
) -> Result<(PathBuf, String)> {
    let text = order(text, rows)?;
    let mut blocks: Vec<(String, String)> = Vec::new();
    for m in rows {
        let name = block_name(m.row.section());
        let body = block(m, rows, rule_count);
        match blocks.iter_mut().find(|b| b.0 == name) {
            Some(b) => b.1 += &body,
            None => blocks.push((name, body)),
        }
    }
    let blocks: Vec<(&str, String)> = blocks
        .iter()
        .map(|(n, b)| (n.as_str(), b.clone()))
        .collect();
    splice_text(PATH, text, &blocks)
}

/// The marker name of the block of a section: `measured-` and its heading in lowercase words joined by `-`.
fn block_name(section: &str) -> String {
    let words: Vec<String> = section
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect();
    format!("measured-{}", words.join("-"))
}

/// `text` with the sections that name a row in the order of `rows` (a section of several rows at the place of its
/// first).
fn order(text: &str, rows: &[Measured]) -> Result<String> {
    let mut sections: Vec<String> = vec![String::new()];
    for line in text.split_inclusive('\n') {
        if line.starts_with("## ") {
            sections.push(String::new());
        }
        sections.last_mut().expect("a section").push_str(line);
    }
    let rank = |section: &str| rows.iter().position(|m| m.row.section() == section);
    let heading = |s: &String| {
        s.lines()
            .next()
            .unwrap_or("")
            .trim_start_matches("## ")
            .to_string()
    };
    for m in rows {
        let name = m.row.section();
        if !sections
            .iter()
            .any(|s| s.starts_with("## ") && heading(s) == name)
        {
            bail!(
                "{PATH}: no `## {name}` section for the row `{}`",
                m.row.language
            );
        }
    }
    let at: Vec<usize> = (0..sections.len())
        .filter(|i| rank(&heading(&sections[*i])).is_some() && sections[*i].starts_with("## "))
        .collect();
    let (first, last) = (at[0], at[at.len() - 1]);
    if at.len() != last - first + 1 {
        bail!(
            "{PATH}: the language sections are not together (a section of no language sits between two)"
        );
    }
    sections[first..=last].sort_by_key(|s| rank(&heading(s)));
    Ok(sections.concat())
}

/// The lines of one row: what each reference tool measured, and its note.
fn block(m: &Measured, rows: &[Measured], rule_count: &dyn Fn(&[String]) -> usize) -> String {
    let shared = rows
        .iter()
        .filter(|r| r.row.section() == m.row.section())
        .count()
        > 1;
    let mut out = format!(
        "- **Measured{}:** {}.\n",
        if shared {
            format!(" ({})", m.row.language)
        } else {
            String::new()
        },
        m.status
    );
    let row = m.row;
    let stability = Tool {
        name: "stable".into(),
        ..Tool::default()
    };
    // The stability share, when the row has one, follows the reference formatters.
    let format: Vec<&Tool> = row
        .format
        .iter()
        .chain((!row.stability.is_empty()).then_some(&stability))
        .collect();
    for ((t, n), share) in format.iter().zip(&m.format_n).zip(&m.format) {
        let (what, unit) = match t.name.as_str() {
            "stable" => (
                "of the files format stably, by the rules of the formatter",
                "files",
            ),
            _ => ("of the files identical", "files"),
        };
        out += &line("Format", &t.name, *share, *n, what, unit);
    }
    if row.format.is_empty() && row.stability.is_empty() {
        out += "  - Format: Sudsy does not format the language.\n";
    }
    for ((t, n), share) in row.lint.iter().zip(&m.lint_n).zip(&m.lint) {
        out += &line(
            "Lint",
            &t.name,
            *share,
            *n,
            "of its findings also reported",
            "findings",
        );
    }
    if row.lint.is_empty() {
        out += &format!(
            "  - Lint: {} rules, no reference tool to compare with.\n",
            rule_count(&row.ids)
        );
    }
    for t in row
        .format
        .iter()
        .chain(&row.lint)
        .filter(|t| t.note.is_some())
    {
        out += &format!("  - {}: {}.\n", t.name, t.note.as_deref().unwrap_or(""));
    }
    out
}

/// `  - Format, Prettier: 95% of the files identical (4,888 of 5,338 files).`; the counts when known.
fn line(
    leg: &str,
    tool: &str,
    share: Option<f64>,
    n: Option<(u64, u64)>,
    what: &str,
    unit: &str,
) -> String {
    let Some(share) = share else {
        return format!("  - {leg}, {tool}: not measured.\n");
    };
    let counts = n.map_or(String::new(), |(a, b)| {
        format!(" ({} of {} {unit})", thousands(a), thousands(b))
    });
    format!("  - {leg}, {tool}: {}% {what}{counts}.\n", share.floor())
}

/// `4888` as `4,888`.
fn thousands(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thousands_groups_by_three() {
        assert_eq!(thousands(7), "7");
        assert_eq!(thousands(4888), "4,888");
        assert_eq!(thousands(1234567), "1,234,567");
    }
}
