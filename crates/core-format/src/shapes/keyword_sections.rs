//! The arguments of a command whose signature is known, grouped into sections: a keyword and the
//! values that follow it (one, many, `name value` pairs, or the words of a command line), the
//! keywords inside a section that opens one, and a first word or two that choose a form
//! (`APPEND list`). The call stays on one line when it fits; else every section is a line of its
//! own, indented, and a section that does not fit goes one value per line (a command line is
//! filled). A call with a list of more than four values breaks even when it would fit. The
//! closing parenthesis stands alone. A call with a comment inside is not laid out here.
//!
//! concept: keyword-sections
//! what: Lays out the arguments of a call by its command's signature: keyword sections, forms chosen by the first words, pairs and command lines, one line when it fits.
//! tools: gersemi keyword sections
//! reads: path:signature::Signature, option:leading_argument_commands

use crate::FormatCtx;
use crate::comments;
use crate::doc;
use crate::doc::Doc;
use crate::error::FormatError;
use core_syntax::signature::{Form, Keyword, Signature, Values, find};
use core_syntax::{SyntaxElement, SyntaxNode, SyntaxToken};

/// More values than this in a section or after the leading ones break the call.
const LONGEST_INLINE: usize = 4;

/// One argument: its document and, when it is a bare word, that word.
struct Item {
    doc: Doc,
    word: Option<String>,
    /// A comment stands in front of it or after it.
    commented: bool,
}

/// A group of arguments laid out as one line when it fits.
enum Section {
    Plain(usize),
    /// Arguments that stay on one line: what stands in front of the first operator.
    Run(Vec<usize>),
    /// A keyword, its values and the sections inside it.
    Keyword(&'static Keyword, usize, Vec<usize>, Vec<Section>),
}

fn commented(cx: &FormatCtx<'_>, node: &SyntaxNode) -> bool {
    let g = cx.grammar;
    cx.first_token(node)
        .is_some_and(|t| !comments::leading_comments(g, &t).is_empty())
        || cx
            .last_token(node)
            .is_some_and(|t| !comments::trailing_comments(g, &t).is_empty())
}

fn bare_word(cx: &FormatCtx<'_>, arg: &SyntaxNode) -> Option<String> {
    let mut tokens = arg
        .children_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .filter(|t| !cx.grammar.is_trivia(t.kind()));
    let first: SyntaxToken = tokens.next()?;
    let alone = tokens.next().is_none() && !cx.grammar.has_tag(first.kind(), "literal");
    alone.then(|| first.text().to_string())
}

/// The form whose words start the call, with the number of items they take.
fn form_of<'a>(sig: &'a Signature, items: &[Item]) -> Option<&'a Form> {
    let word = |i: usize| items.get(i).and_then(|x| x.word.as_deref());
    sig.forms
        .iter()
        .filter(|f| {
            f.words
                .iter()
                .enumerate()
                .all(|(i, w)| word(i).is_some_and(|x| x.eq_ignore_ascii_case(w)))
        })
        .max_by_key(|f| f.words.len())
}

/// The sections of `items[from..]` read with `keywords`; a keyword of `outer` ends a section that
/// opened inside.
fn sections(
    items: &[Item],
    from: usize,
    keywords: &[&'static [Keyword]],
    outer: &[&'static [Keyword]],
) -> (Vec<Section>, usize) {
    let in_sets = |sets: &[&'static [Keyword]], w: &str| sets.iter().find_map(|k| find(k, w));
    let is_keyword = |i: usize| {
        items[i]
            .word
            .as_deref()
            .and_then(|w| in_sets(keywords, w).or_else(|| in_sets(outer, w)))
    };
    let own = |i: usize| items[i].word.as_deref().and_then(|w| in_sets(keywords, w));
    let mut out = Vec::new();
    let mut i = from;
    while i < items.len() {
        let Some(k) = own(i) else {
            if !outer.is_empty() && is_keyword(i).is_some() {
                break;
            }
            out.push(Section::Plain(i));
            i += 1;
            continue;
        };
        let at = i;
        i += 1;
        let mut values = Vec::new();
        match k.values {
            Values::None => {}
            Values::One => {
                if i < items.len() && is_keyword(i).is_none() {
                    values.push(i);
                    i += 1;
                }
            }
            Values::Many | Values::Pairs | Values::CommandLine | Values::Infix => {
                while i < items.len() && is_keyword(i).is_none() {
                    values.push(i);
                    i += 1;
                }
            }
        }
        let mut inner = Vec::new();
        if !k.inner.is_empty() {
            let mut chain = keywords.to_vec();
            chain.extend_from_slice(outer);
            let (s, next) = sections(items, i, &[k.inner], &chain);
            inner = s;
            i = next;
        }
        out.push(Section::Keyword(k, at, values, inner));
    }
    (out, i)
}

/// The words of a form that has no positional argument open a section of their own.
fn lead_section(items: &[Item], keywords: &[&'static [Keyword]]) -> (Section, usize) {
    static LEAD: Keyword = Keyword {
        word: "",
        values: Values::Many,
        inner: &[],
    };
    let mut i = 1;
    let mut values = Vec::new();
    while i < items.len()
        && !items[i]
            .word
            .as_deref()
            .is_some_and(|w| keywords.iter().any(|k| find(k, w).is_some()))
    {
        values.push(i);
        i += 1;
    }
    (Section::Keyword(&LEAD, 0, values, Vec::new()), i)
}

/// The sections of a command whose keywords are operators: a line in front of the first one, then
/// one section per operator with the operands up to the next.
fn operator_sections(items: &[Item], keywords: &[&'static [Keyword]]) -> Vec<Section> {
    let op = |i: usize| {
        let word = items[i].word.as_deref()?;
        let k = keywords.iter().find_map(|set| find(set, word))?;
        (k.values == Values::Infix).then_some(k)
    };
    let operands = |mut i: usize| {
        let mut v = Vec::new();
        while i < items.len() && op(i).is_none() {
            v.push(i);
            i += 1;
        }
        v
    };
    let mut out = Vec::new();
    let lead = operands(0);
    let mut i = lead.len();
    if !lead.is_empty() {
        out.push(Section::Run(lead));
    }
    while let Some(k) = (i < items.len()).then(|| op(i)).flatten() {
        let values = operands(i + 1);
        let next = i + 1 + values.len();
        out.push(Section::Keyword(k, i, values, Vec::new()));
        i = next;
    }
    out
}

fn join(parts: Vec<Doc>) -> Doc {
    doc::join(&doc::line(), parts)
}

fn section_doc(items: &[Item], s: &Section) -> Doc {
    match s {
        Section::Plain(i) => items[*i].doc.clone(),
        Section::Run(run) => {
            let docs: Vec<Doc> = run.iter().map(|i| items[*i].doc.clone()).collect();
            match docs.split_first() {
                Some((first, rest)) if !rest.is_empty() => doc::group(doc::concat(vec![
                    first.clone(),
                    doc::indent(doc::concat(vec![doc::line(), join(rest.to_vec())])),
                ])),
                _ => join(docs),
            }
        }
        Section::Keyword(k, at, values, inner) => {
            let word = items[*at].doc.clone();
            let mut body: Vec<Doc> = Vec::new();
            match k.values {
                Values::Pairs => {
                    for pair in values.chunks(2) {
                        let docs: Vec<Doc> = pair.iter().map(|i| items[*i].doc.clone()).collect();
                        body.push(match docs.split_first() {
                            Some((key, value)) if !value.is_empty() => {
                                doc::group(doc::concat(vec![
                                    key.clone(),
                                    doc::indent(doc::concat(vec![
                                        doc::line(),
                                        join(value.to_vec()),
                                    ])),
                                ]))
                            }
                            _ => join(docs),
                        });
                    }
                }
                Values::CommandLine => {
                    // A word with a comment stands on a line of its own.
                    let mut parts = Vec::new();
                    for (n, i) in values.iter().enumerate() {
                        if n > 0 {
                            let apart = items[*i].commented || items[values[n - 1]].commented;
                            parts.push(if apart { doc::hardline() } else { doc::line() });
                        }
                        parts.push(items[*i].doc.clone());
                    }
                    if !parts.is_empty() {
                        body.push(doc::fill(parts));
                    }
                }
                _ => body.extend(values.iter().map(|i| items[*i].doc.clone())),
            }
            body.extend(inner.iter().map(|s| section_doc(items, s)));
            if body.is_empty() {
                return word;
            }
            doc::group(doc::concat(vec![
                word,
                doc::indent(doc::concat(vec![doc::line(), join(body)])),
            ]))
        }
    }
}

/// The parenthesized arguments of a call to the command `sig`, or `None` for a call with no
/// argument, a comment on the parenthesis or a signature that says nothing.
pub fn keyword_call(
    cx: &FormatCtx<'_>,
    args: &SyntaxNode,
    open: &SyntaxToken,
    close: &SyntaxToken,
    sig: &'static Signature,
) -> Result<Option<Doc>, FormatError> {
    let g = cx.grammar;
    let bare = sig.positional.is_empty() && sig.keywords.is_empty() && sig.forms.is_empty();
    if bare
        || !comments::trailing_comments(g, open).is_empty()
        || !comments::leading_comments(g, close).is_empty()
    {
        return Ok(None);
    }
    let mut items = Vec::new();
    for arg in args.children() {
        items.push(Item {
            doc: cx.format_known(&arg, false)?,
            word: bare_word(cx, &arg),
            commented: commented(cx, &arg),
        });
    }
    if items.is_empty() {
        return Ok(None);
    }
    let form = form_of(sig, &items);
    // A form with positional arguments keeps them with its words; one without has its words open
    // a section of their own.
    let (head, form_keywords): (Vec<usize>, &'static [Keyword]) = match form {
        Some(f) if f.positional > 0 => {
            let n = f.words.len() + usize::from(items.len() > f.words.len());
            ((0..n).collect(), f.keywords)
        }
        Some(f) => (Vec::new(), f.keywords),
        None => (Vec::new(), sig.keywords),
    };
    let lead = form.is_some_and(|f| f.positional == 0);
    let keywords = [form_keywords, sig.keywords];
    let mut rest = Vec::new();
    let mut at = head.len();
    if lead {
        let (section, next) = lead_section(&items, &keywords);
        rest.push(section);
        at = next;
    }
    if form.is_none()
        && keywords
            .iter()
            .any(|set| set.iter().any(|k| k.values == Values::Infix))
    {
        rest = operator_sections(&items, &keywords);
    } else {
        let (more, _) = sections(&items, at, &keywords, &[]);
        rest.extend(more);
    }
    let first_hug = head.is_empty()
        && !lead
        && cx
            .options
            .strings(core_config::opt::LEADING_ARGUMENT_COMMANDS)
            .any(|c| c.eq_ignore_ascii_case(sig.name))
        && rest.len() > 1
        && matches!(rest.first(), Some(Section::Plain(_)));
    let plain = rest
        .iter()
        .filter(|s| matches!(s, Section::Plain(_)))
        .count();
    let force = (plain.saturating_sub(sig.positional.len().max(usize::from(first_hug)))
        > LONGEST_INLINE
        && plain == rest.len())
        || (!head.is_empty() && plain > LONGEST_INLINE);
    let mut docs: Vec<Doc> = Vec::new();
    if !head.is_empty() {
        let words: Vec<Doc> = head.iter().map(|i| items[*i].doc.clone()).collect();
        docs.push(doc::join(&doc::lit(" "), words));
    }
    for (n, s) in rest.iter().enumerate() {
        if first_hug && n == 0 {
            continue;
        }
        docs.push(section_doc(&items, s));
    }
    let body = join(docs);
    let inner = if first_hug {
        doc::concat(vec![
            items[0].doc.clone(),
            doc::indent(doc::concat(vec![doc::line(), body])),
            doc::softline(),
        ])
    } else {
        doc::concat(vec![
            doc::indent(doc::concat(vec![doc::softline(), body])),
            doc::softline(),
        ])
    };
    let breaker = if force {
        doc::break_parent()
    } else {
        doc::nil()
    };
    Ok(Some(doc::group(doc::concat(vec![
        doc::lit("("),
        breaker,
        inner,
        doc::lit(")"),
    ]))))
}
