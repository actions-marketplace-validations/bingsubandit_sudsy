//! The long texts of a rule, read when a page, `sudsy rules` or the rule list asks and never by a
//! run that only lints: the `description` and `why` blocks of a rule file are folded from the
//! file's text then (`block`), and the `why` of a rule that several languages implement with one
//! reason is a section of `texts/why.txt`, which a provider's own `why` overrides.
//!
//! concept: none (infrastructure: shared rule texts)

use std::collections::HashMap;
use std::sync::OnceLock;

const TEXTS: &str = include_str!("../texts/why.txt");

/// The sections of the file by rule id, split on first use.
fn sections() -> &'static HashMap<&'static str, &'static str> {
    static MAP: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    MAP.get_or_init(|| {
        TEXTS
            .split("\n== ")
            .skip(1)
            .filter_map(|s| s.split_once('\n'))
            .map(|(id, body)| (id.trim(), body))
            .collect()
    })
}

/// The shared `why` of the rule `id`: its paragraphs, one per line.
pub fn shared_why(id: &str) -> Option<String> {
    let body = sections().get(id)?;
    let paragraphs: Vec<String> = body
        .split("\n\n")
        .map(|p| p.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|p| !p.is_empty())
        .collect();
    Some(paragraphs.join("\n"))
}

/// The value of `key` in a rule file's text: a folded block `>-` (a line break is a space, a
/// blank line a line break) or a one-line scalar that reads the same in every YAML way of writing
/// it. `None` when the file has no such key or writes it another way.
pub fn block(source: &str, key: &str) -> Option<String> {
    let mut lines = source
        .lines()
        .skip_while(|l| !(l.starts_with(key) && l[key.len()..].starts_with(':')));
    let value = lines.next()?[key.len() + 1..].trim();
    if value != ">-" {
        let quoted = |q: char| {
            value
                .strip_prefix(q)?
                .strip_suffix(q)
                .filter(|v| !v.contains(['\\', q]))
        };
        return quoted('\'')
            .or_else(|| quoted('"'))
            .or(Some(value).filter(|v| !v.starts_with(['\'', '"', '|', '>'])))
            .map(str::to_string);
    }
    let (mut out, mut blanks) = (String::new(), 0);
    for line in lines {
        if line.trim().is_empty() {
            blanks += 1;
        } else if line.starts_with(' ') {
            match (out.is_empty(), blanks) {
                (true, _) => {}
                (false, 0) => out.push(' '),
                _ => out.push('\n'),
            }
            out.push_str(line.trim());
            blanks = 0;
        } else {
            break;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::Category;
    use crate::rule::{Level, RuleMeta};

    fn meta(id: &'static str, why: &'static str, source: &'static str) -> RuleMeta {
        RuleMeta {
            id,
            category: Category::Style,
            description: "Reports a demo.",
            why,
            source,
            example: "",
            default_level: Level::Off,
            fix: None,
            requires: &[],
            uses: &[],
            options: &[],
        }
    }

    #[test]
    fn a_shared_text_reaches_a_rule_without_its_own_and_an_own_text_wins() {
        let shared = meta("no-regex-spaces", "", "").why_text();
        assert!(shared.starts_with("Several spaces in a row"), "{shared}");
        assert!(
            shared.contains("\nTo fix, write the number as a count"),
            "{shared}"
        );
        assert_eq!(meta("no-regex-spaces", "its own", "").why_text(), "its own");
        let file = "id: x\nwhy: >-\n  Own text\n  goes on.\n\n  Second.\nexample: y\n";
        assert_eq!(
            meta("no-regex-spaces", "", file).why_text(),
            "Own text goes on.\nSecond."
        );
        assert_eq!(meta("no-such-rule", "", "").why_text(), "");
    }

    #[test]
    fn a_block_is_read_from_the_file_text() {
        let file = "id: x\ndescription: >-\n  One\n  two.\nwhy: z\n";
        assert_eq!(block(file, "description").as_deref(), Some("One two."));
        assert_eq!(block(file, "why").as_deref(), Some("z"));
        assert_eq!(block(file, "example"), None);
        assert_eq!(block("a: 'b c'\nd: \"e\"\n", "d").as_deref(), Some("e"));
    }
}
