//! The formatter: sqruff's own fixer run with sqlfluff's layout rules (LT01-LT15) and
//! capitalisation rules (CP01-CP05), configured from the format options. sqruff parses again
//! (its fixer works on its own segments), so the result is text that `format_parsed` parses once
//! more for the safety check.
//!
//! sqruff's fixer has no guard against a fix that changes the code or leaves the grammar (and the
//! engine checks the output against the rewritten tree only), so every fix is verified against
//! the input: when the whole file fails, each statement is fixed and verified on its own and the
//! ones that fail stay as written.

use crate::format::{
    COMMA_POSITION, FUNCTION_CASE, IDENTIFIER_CASE, KEYWORD_CASE, LITERAL_CASE, TYPE_CASE,
};
use crate::format::{DIALECT, SqlFormatter};
use core_config::ResolvedOptions;
use core_format::safety;
use core_syntax::{Grammar, SyntaxElement, SyntaxNode};
use sqruff_lib::core::config::FluffConfig;
use sqruff_lib::core::linter::core::Linter;
use std::cell::RefCell;
use std::fmt::Write as _;
use std::panic::{AssertUnwindSafe, catch_unwind};

/// (CP rule code, option, sqruff's rule section, its policy key)
const CASES: [(&str, &str, &str, &str); 5] = [
    ("CP01", KEYWORD_CASE, "keywords", "capitalisation_policy"),
    (
        "CP02",
        IDENTIFIER_CASE,
        "identifiers",
        "extended_capitalisation_policy",
    ),
    (
        "CP03",
        FUNCTION_CASE,
        "functions",
        "extended_capitalisation_policy",
    ),
    ("CP04", LITERAL_CASE, "literals", "capitalisation_policy"),
    ("CP05", TYPE_CASE, "types", "extended_capitalisation_policy"),
];

/// sqruff's configuration (INI) for the options.
fn config(dialect: &str, o: &ResolvedOptions) -> String {
    let mut rules: Vec<String> = (1..=15).map(|n| format!("LT{n:02}")).collect();
    let mut ini = String::new();
    for (code, option, section, key) in CASES {
        let policy = o.str(option);
        if policy != "preserve" {
            rules.push(code.into());
            let _ = writeln!(
                ini,
                "[sqruff:rules:capitalisation.{section}]\n{key} = {policy}"
            );
        }
    }
    let unit = if o.str("indent_style") == "tab" {
        "tab"
    } else {
        "space"
    };
    format!(
        "[sqruff]\ndialect = {dialect}\nrules = {}\nmax_line_length = {}\n\
         large_file_skip_byte_limit = 0\n[sqruff:indentation]\nindent_unit = {unit}\n\
         tab_space_size = {}\n[sqruff:layout:type:comma]\nline_position = {}\n{ini}",
        rules.join(","),
        o.int("line_width"),
        o.int("indent_size"),
        o.str(COMMA_POSITION),
    )
}

thread_local! {
    // The last linter of the thread: building one parses sqruff's default configuration.
    static LINTER: RefCell<Option<(String, Linter)>> = const { RefCell::new(None) };
}

/// `src` fixed, or `None` when sqruff panics or fails on it. `src` has LF line breaks.
fn fix(src: &str, dialect: &str, o: &ResolvedOptions) -> Option<String> {
    let ini = config(dialect, o);
    catch_unwind(AssertUnwindSafe(|| {
        LINTER.with_borrow_mut(|slot| {
            if slot.as_ref().is_none_or(|(have, _)| *have != ini) {
                let config = FluffConfig::from_source(&ini, None);
                *slot = Some((ini, Linter::new(config, None, None, false).ok()?));
            }
            let (_, linter) = slot.as_ref()?;
            Some(linter.lint_string(src, None, true).ok()?.fix_string())
        })
    }))
    .ok()
    .flatten()
}

/// `text` fixed and verified against `tree` (its parse), or `None`.
fn verified(
    f: &SqlFormatter,
    tree: &SyntaxNode,
    text: &str,
    grammar: &Grammar,
    o: &ResolvedOptions,
) -> Option<String> {
    let dialect = o.str(DIALECT);
    let fixed = fix(text, dialect, o)?;
    let parse = |text: &str| {
        let parsed = crate::parse(text, dialect);
        match parsed.errors.first() {
            Some(e) => Err(e.message.clone()),
            None => Ok(parsed.tree),
        }
    };
    safety::verify(grammar, f, tree, &fixed, parse).ok()?;
    Some(fixed)
}

/// The statements of a file as texts: each ends with its `;` and the trivia after it.
fn statements(tree: &SyntaxNode, grammar: &Grammar) -> Vec<String> {
    let (mut out, mut cur, mut after_semicolon) = (Vec::new(), String::new(), false);
    for el in tree.children_with_tokens() {
        if after_semicolon && !grammar.is_trivia(el.kind()) {
            out.push(std::mem::take(&mut cur));
            after_semicolon = false;
        }
        after_semicolon |= el.kind() == crate::generated::kind::SEMICOLON;
        match &el {
            SyntaxElement::Token(t) => cur.push_str(t.text()),
            SyntaxElement::Node(n) => cur.push_str(&n.text().to_string()),
        }
    }
    out.push(cur);
    out
}

/// The source to format instead of `text` (the engine's `rewrite_source`).
pub fn rewrite(
    f: &SqlFormatter,
    tree: &SyntaxNode,
    text: &str,
    grammar: &Grammar,
    o: &ResolvedOptions,
) -> Option<String> {
    let lf = text.replace("\r\n", "\n");
    let fixed = verified(f, tree, &lf, grammar, o).or_else(|| {
        let dialect = o.str(DIALECT);
        let mut joined = String::new();
        for statement in statements(tree, grammar) {
            // The blank lines before a statement are the file's layout, not the statement's.
            let blank = statement.len() - statement.trim_start_matches(['\n', ' ', '\t']).len();
            let body_at = statement[..blank].rfind('\n').map_or(0, |i| i + 1);
            let (lead, body) = statement.split_at(body_at);
            let parsed = crate::parse(body, dialect);
            let fixed = parsed
                .errors
                .is_empty()
                .then(|| verified(f, &parsed.tree, body, grammar, o))
                .flatten();
            joined.push_str(lead);
            joined.push_str(fixed.as_deref().unwrap_or(body));
        }
        let parse = |text: &str| {
            let parsed = crate::parse(text, dialect);
            parsed
                .errors
                .first()
                .map_or(Ok(parsed.tree.clone()), |e| Err(e.message.clone()))
        };
        safety::verify(grammar, f, tree, &joined, parse).ok()?;
        Some(joined)
    })?;
    // The engine takes `end_of_line: auto` from the text it formats.
    let crlf = text.find('\n').is_some_and(|i| text[..i].ends_with('\r'));
    Some(if crlf {
        fixed.replace('\n', "\r\n")
    } else {
        fixed
    })
}
