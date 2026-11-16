//! Capitalisation (sqlfluff's CP01 to CP05): keywords (and binary operators), unquoted identifiers,
//! function names, `NULL`/`TRUE`/`FALSE` and data types each have a policy. `consistent` follows
//! sqlfluff: the first word fixes what the file may be (`upper`, `lower` or `capitalise`), each
//! word that differs rules cases out, and once none is left the words after it, and so the whole
//! file, take the case the last non-empty set preferred.

use super::seq::Seq;
use crate::generated::kind as k;
use core_config::{OptionValue, ResolvedOptions};

const UPPER: u8 = 1;
const LOWER: u8 = 2;
const CAPITALISE: u8 = 4;
/// Preference order of the cases a file may still be in.
const ORDER: [u8; 3] = [UPPER, LOWER, CAPITALISE];

#[derive(Clone, Copy)]
enum Policy {
    Consistent,
    Fixed(u8),
}

#[derive(Default)]
struct Memory {
    refuted: u8,
    latest: Option<u8>,
}

fn capitalise(s: &str) -> String {
    let mut chars = s.chars();
    chars.next().map_or_else(String::new, |c| {
        c.to_uppercase()
            .chain(chars.flat_map(char::to_lowercase))
            .collect()
    })
}

fn convert(raw: &str, case: u8) -> String {
    match case {
        UPPER => raw.to_uppercase(),
        LOWER => raw.to_lowercase(),
        _ => capitalise(raw),
    }
}

impl Memory {
    /// Takes in `raw`; returns the case it must be written in, if it is not already.
    fn visit(&mut self, raw: &str, policy: Policy) -> Option<String> {
        let first_lower = raw
            .chars()
            .find(|c| c.to_lowercase().ne(c.to_uppercase()))
            .map(|c| c.to_uppercase().ne(std::iter::once(c)));
        if first_lower == Some(true) {
            self.refuted |= UPPER | CAPITALISE;
            if raw != raw.to_lowercase() {
                self.refuted |= LOWER;
            }
        } else {
            self.refuted |= LOWER;
            if raw != raw.to_uppercase() {
                self.refuted |= UPPER;
            }
            if raw != capitalise(raw) {
                self.refuted |= CAPITALISE;
            }
        }
        let case = match policy {
            Policy::Fixed(case) if self.refuted & case == 0 => return None,
            Policy::Fixed(case) => case,
            Policy::Consistent => {
                if let Some(c) = ORDER.into_iter().find(|c| self.refuted & c == 0) {
                    self.latest = Some(c);
                    return None;
                }
                self.latest.unwrap_or(UPPER)
            }
        };
        let fixed = convert(raw, case);
        (fixed != raw).then_some(fixed)
    }
}

fn policy(options: &ResolvedOptions, name: &str) -> Policy {
    match options.get(name) {
        Some(OptionValue::String(s)) => match s.as_str() {
            "upper" => Policy::Fixed(UPPER),
            "lower" => Policy::Fixed(LOWER),
            "capitalise" => Policy::Fixed(CAPITALISE),
            _ => Policy::Consistent,
        },
        _ => Policy::Consistent,
    }
}

/// Rewrites the case of the words of the file.
pub fn recase(seq: &mut Seq, options: &ResolvedOptions) {
    let names = [
        "keyword_case",
        "identifier_case",
        "function_case",
        "literal_case",
        "type_case",
    ];
    let policies = names.map(|n| policy(options, n));
    let mut memory: [Memory; 5] = Default::default();
    for block in &mut seq.blocks {
        let class = match block.kind {
            k::KEYWORD | k::BINARY_KEYWORD | k::BINARY_OPERATOR => 0,
            k::IDENTIFIER => 1,
            k::FUNCTION_IDENTIFIER => 2,
            k::WORD_LITERAL => 3,
            k::TYPE_IDENTIFIER => 4,
            _ => continue,
        };
        if let Some(fixed) = memory[class].visit(&block.text, policies[class]) {
            block.text = fixed;
        }
    }
}
