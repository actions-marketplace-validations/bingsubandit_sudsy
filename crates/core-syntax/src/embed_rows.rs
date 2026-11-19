//! The `embed` rows of a grammar: which node or token holds code of another language, how that
//! code is carved out of the host's text and which language it is. One finder reads them
//! (`find`); the host's language crate states facts only.
//!
//! `embed <Kind> language "<name>" [attribute...]` where `Kind` is the node or token that holds
//! the code. Attributes:
//! - `in <Kind>`: its direct parent; `lead "w"...`: the parent's first token is one of the words
//!   (any case); `named <Token> "w"`: the first token of that kind inside it has that text;
//!   `root`: it is a child of the tree's root; `dialect "d"`: only for that dialect's provider;
//!   `when "s"...`: only a file that contains one of the strings;
//! - `skip-start "s"...`: a token whose text starts so is not code;
//! - the region: the whole text, `content <Kind>` (the first child of that kind), `body` (the lines
//!   strictly between the first and last line of a heredoc), `trim` (first to last token that is
//!   not trivia), `fence <Token>` (the content lines of a fenced block opened by that token, inside
//!   its `container`s: their prefixes are carved), `after <Token>` (the region starts after the last own
//!   token of that kind); `join`: all matches of the row are one region, the text between them is carved;
//! - the language: `info <Token>` (the first word of that token, `alias "a" "b"` renames),
//!   `attr "name" "l"...` (the value of that attribute of the `named` node, one of `l`; absent
//!   it is `language`, anything else is no code);
//! - the host's parts: `carve-lines "p"...` (lines after the first that start with `p`),
//!   `carve-end "c" [directive "name"]` (a line's last character `c` and its break, when the
//!   file's leading `# name=c` directive names it), `carve-line-start "s"` (`s` after a break),
//!   `carve-kind <Kind>...` (every node or token of the kind), `carve-first <Kind>...` (the first
//!   character of each);
//! - `format`, `strip`, `no-lint`: the [`Embedded`] flags.
//!
//! concept: embed-rows
//! what: Which node holds code of another language and how it is carved out of the host text.
//! tools: none (embedded code as rows)
//! reads: grammar:embed

use crate::containers::Containers;
use crate::embedded::size;
use crate::tree::{SyntaxElement, SyntaxNode, SyntaxNodeExt, SyntaxToken, code_span};
use crate::{Embedded, Grammar, SyntaxKind, TextRange, TextSize};

#[derive(Clone, Debug, Default)]
pub struct EmbedRow {
    pub kind: Option<SyntaxKind>,
    pub language: String,
    pub parent: Option<SyntaxKind>,
    pub lead: Vec<String>,
    pub named: Option<(SyntaxKind, String)>,
    pub root: bool,
    pub dialect: Option<String>,
    pub when: Vec<String>,
    pub skip_start: Vec<String>,
    pub content: Option<SyntaxKind>,
    pub fence: Option<SyntaxKind>,
    pub after: Option<SyntaxKind>,
    pub body: bool,
    pub trim: bool,
    pub join: bool,
    pub info: Option<SyntaxKind>,
    pub alias: Vec<(String, String)>,
    pub attr: Option<(String, Vec<String>)>,
    pub carve_lines: Vec<String>,
    pub carve_end: Option<(char, Option<String>)>,
    pub carve_line_start: Vec<String>,
    pub carve_kind: Vec<SyntaxKind>,
    pub carve_first: Vec<SyntaxKind>,
    pub format: bool,
    pub strip: bool,
    pub no_lint: bool,
}

/// The code the rows of `grammar` find in `root` (`dialect`: the provider's dialect name), in one
/// walk over the nodes. A row of a token kind names the `in` node that holds the token.
pub fn find(
    grammar: &Grammar,
    dialect: Option<&str>,
    root: &SyntaxNode,
    text: &str,
) -> Vec<Embedded> {
    let rows: Vec<(usize, &EmbedRow)> = (grammar.embed_rows().iter().enumerate())
        .filter(|(_, r)| {
            r.kind.is_some()
                && r.dialect.as_deref().is_none_or(|d| Some(d) == dialect)
                && (r.when.is_empty() || r.when.iter().any(|w| text.contains(w.as_str())))
        })
        .collect();
    if rows.is_empty() {
        return Vec::new();
    }
    let mut found: Vec<(usize, Embedded)> = Vec::new();
    let mut joined: Vec<Vec<(TextRange, Option<String>)>> =
        rows.iter().map(|_| Vec::new()).collect();
    let mut visit = |n: &SyntaxNode, at_root: bool| {
        for (slot, &(index, row)) in rows.iter().enumerate() {
            let kind = row.kind.expect("filtered");
            if row.root && !at_root {
                continue;
            }
            let hosts: Vec<SyntaxElement> = if !kind.is_token() {
                if n.kind() == kind {
                    vec![SyntaxElement::Node(n.clone())]
                } else {
                    continue;
                }
            } else if row.parent == Some(n.kind()) {
                (n.children_with_tokens())
                    .filter(|e| e.kind() == kind)
                    .collect()
            } else {
                continue;
            };
            for host in hosts {
                let Some((range, language, carved)) = region(grammar, row, &host, text) else {
                    continue;
                };
                if row.join {
                    joined[slot].push((range, language));
                } else {
                    found.push((index, embedded(row, &host, language, range, carved, text)));
                }
            }
        }
    };
    if rows.iter().all(|(_, r)| r.root) {
        root.children().for_each(|n| visit(&n, true));
    } else {
        root.descendants()
            .for_each(|n| visit(&n, n.parent().is_some_and(|p| p == *root)));
    }
    for (slot, &(index, row)) in rows.iter().enumerate() {
        let joined = &joined[slot];
        if let (Some(first), Some(last)) = (joined.first(), joined.last()) {
            let gaps = (joined.windows(2))
                .map(|w| TextRange::new(w[0].0.end(), w[1].0.start()))
                .collect();
            let whole = TextRange::new(first.0.start(), last.0.end());
            let language = joined.iter().find_map(|j| j.1.as_deref());
            let mut region = Embedded::new(language.unwrap_or(&row.language), whole, gaps);
            flags(row, &mut region);
            found.push((index, region));
        }
    }
    found.sort_by_key(|f| f.0);
    found.into_iter().map(|f| f.1).collect()
}

fn flags(row: &EmbedRow, region: &mut Embedded) {
    region.format = row.format;
    region.strip = row.strip;
    region.lint = !row.no_lint;
}

fn embedded(
    row: &EmbedRow,
    host: &SyntaxElement,
    language: Option<String>,
    range: TextRange,
    mut carved: Vec<TextRange>,
    text: &str,
) -> Embedded {
    let body = &text[range];
    let from = usize::from(range.start());
    let at = |i: usize| size(from + i);
    let lines = || {
        let mut start = 0;
        body.split_inclusive('\n').map(move |l| {
            let s = start;
            start += l.len();
            (s, l)
        })
    };
    for (i, line) in lines().skip(1) {
        if row
            .carve_lines
            .iter()
            .any(|p| line.trim_start().starts_with(p.as_str()))
        {
            carved.push(TextRange::new(at(i), at(i + line.len())));
        }
    }
    if let Some((c, directive)) = &row.carve_end
        && directive
            .as_ref()
            .is_none_or(|d| directive_value(text, d) == Some(*c))
    {
        for (i, line) in lines() {
            let code = line.trim_end();
            if code.ends_with(*c) {
                carved.push(TextRange::new(
                    at(i + code.len() - c.len_utf8()),
                    at(i + line.len()),
                ));
            }
        }
    }
    for s in &row.carve_line_start {
        for (i, _) in body.match_indices(&format!("\n{s}")) {
            carved.push(TextRange::at(at(i + 1), size(s.len())));
        }
    }
    if !(row.carve_kind.is_empty() && row.carve_first.is_empty())
        && let Some(node) = host.as_node()
    {
        for e in node.descendants_with_tokens() {
            let r = e.text_range();
            if row.carve_kind.contains(&e.kind()) {
                carved.push(r);
            }
            if row.carve_first.contains(&e.kind()) {
                carved.push(TextRange::at(r.start(), TextSize::from(1)));
            }
        }
    }
    let mut region = Embedded::new(language.as_deref().unwrap_or(&row.language), range, carved);
    flags(row, &mut region);
    region
}

/// The value of the file's leading parser directive `# name=value` (a single character).
fn directive_value(text: &str, name: &str) -> Option<char> {
    text.lines()
        .map(|l| {
            l.trim()
                .strip_prefix('#')
                .map(|d| d.trim().replace(' ', ""))
        })
        .take_while(|d| d.as_ref().is_some_and(|d| d.contains('=')))
        .flatten()
        .find_map(|d| {
            let (k, v) = d.split_once('=')?;
            let mut chars = v.chars();
            (k.eq_ignore_ascii_case(name) && chars.clone().count() == 1).then(|| chars.next())?
        })
}

/// The region of `host` and its language, when the row's conditions hold.
type Region = (TextRange, Option<String>, Vec<TextRange>);

fn region(grammar: &Grammar, row: &EmbedRow, host: &SyntaxElement, text: &str) -> Option<Region> {
    let parent = host.parent();
    if let Some(kind) = row.parent {
        let parent = parent.as_ref().filter(|p| p.kind() == kind)?;
        if !row.lead.is_empty() {
            // The instruction's first token that is not trivia: blank lines and comments before it belong to the node.
            let first = parent.tokens().find(|t| !grammar.is_trivia(t.kind()))?;
            let word = first.text();
            if !row.lead.iter().any(|l| word.eq_ignore_ascii_case(l)) {
                return None;
            }
        }
    }
    if row.root && !parent.as_ref().is_some_and(|p| p.parent().is_none()) {
        return None;
    }
    let mut language = None;
    if let Some(node) = host.as_node() {
        let token_of = |k: SyntaxKind| {
            node.descendants_with_tokens()
                .filter_map(|e| e.into_token())
                .find(|t: &SyntaxToken| t.kind() == k)
        };
        if let Some((k, word)) = &row.named
            && !token_of(*k).is_some_and(|t| t.text().eq_ignore_ascii_case(word))
        {
            return None;
        }
        if let Some((name, allowed)) = &row.attr
            && let Some(value) = attribute(grammar, node, name)
        {
            if !allowed.iter().any(|a| a == &value) {
                return None;
            }
            language = Some(value);
        }
    }
    if let Some(info) = row.info {
        let node = host.as_node()?;
        let word = node.token(info)?;
        let word = word.text().split_whitespace().next()?;
        let word = word.trim_matches(['{', '}', '.']).to_lowercase();
        language = Some(match row.alias.iter().find(|a| a.0 == word) {
            Some(a) => a.1.clone(),
            None => word,
        });
    }
    if let (Some(fence), Some(node)) = (row.fence, host.as_node()) {
        let (lines, closed) = Containers { grammar, src: text }.fence_lines(node, fence);
        let (Some(first), Some(last), true) = (lines.first(), lines.last(), closed) else {
            return None;
        };
        let carved = (lines.iter().filter(|l| l.1 > 0))
            .map(|&(start, prefix, _)| TextRange::new(size(start), size(start + prefix)))
            .collect();
        return Some((
            TextRange::new(size(first.0), size(last.2)),
            language,
            carved,
        ));
    }
    let mut range = match (host, row.content) {
        (SyntaxElement::Node(n), Some(k)) => n.child(k)?.text_range(),
        (SyntaxElement::Node(n), None) if row.trim => code_span(n, grammar)?,
        (e, _) => e.text_range(),
    };
    if let (Some(k), Some(n)) = (row.after, host.as_node())
        && let Some(last) = n.tokens().filter(|t| t.kind() == k).last()
    {
        range = TextRange::new(range.start().max(last.text_range().end()), range.end());
    }
    let body = &text[range];
    if row.skip_start.iter().any(|s| body.starts_with(s.as_str())) {
        return None;
    }
    if row.body {
        let (a, b) = (body.find('\n')?, body.rfind('\n')?);
        if a >= b {
            return None;
        }
        range = TextRange::new(range.start() + size(a + 1), range.start() + size(b));
    }
    if row.trim {
        let src = &text[range];
        let start = range.start() + size(src.len() - src.trim_start().len());
        let end = range.start() + size(src.trim_end().len());
        (start < end).then_some(())?;
        range = TextRange::new(start, end);
    }
    Some((range, language, Vec::new()))
}

/// The value of attribute `name` (lower case) of the entries (`#key-value` children) of the node
/// that holds the first token of the `named` kind; quotes stripped.
fn attribute(grammar: &Grammar, node: &SyntaxNode, name: &str) -> Option<String> {
    let tag = node
        .children()
        .find(|c| grammar.has_tag(c.kind(), "mapping"))?;
    tag.children()
        .filter(|a| grammar.has_tag(a.kind(), "key-value"))
        .find_map(|a| {
            let tokens: Vec<_> = a
                .tokens()
                .filter(|t| !grammar.is_trivia(t.kind()))
                .collect();
            let (key, rest) = tokens.split_first()?;
            if !key.text().eq_ignore_ascii_case(name) {
                return None;
            }
            let value = rest.last().filter(|t| t.text() != "=");
            Some(value.map_or(String::new(), |v| {
                v.text().trim_matches(['"', '\'']).to_string()
            }))
        })
}
