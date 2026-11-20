//! Rules on references written as an anchor and its aliases (YAML `&name` and `*name`) and on the
//! merge key: `no-undefined-alias` (correctness), `no-recursive-alias`, and the opt-in
//! `no-duplicate-anchors`, `no-unused-anchors` and `no-merge-keys`. The grammar tags the `document`
//! nodes and gives the merge key on its `keys` line; the hooks say which tokens are references and
//! the byte each starts with; anchors do not cross documents.
//!
//! concept: reference-rules
//! what: Anchors and their aliases, and the merge key: undefined, recursive, repeated and unused references.
//! tools: yamllint anchors, actionlint anchors
//! reads: grammar:keys, tag:document

use crate::rule;
use crate::unique::Repeats;
use crate::{Edit, FileContext, RuleContext};
use core_syntax::{TextRange, TextSize};
use std::collections::{HashMap, HashSet};

/// An `&name` or `*name` token of the file.
struct Event<'a> {
    anchor: bool,
    name: &'a str,
    range: TextRange,
    /// The start of the document it is in; names are looked up per document.
    document: Option<TextSize>,
    /// For an anchor, when asked for: the node it is on.
    scope: TextRange,
}

/// The anchors and aliases of the file in order. Looks only at the bytes the language's
/// references start with and asks the tree what they are, so a file without references costs one
/// scan and no tree walk.
fn events<'a>(file: &FileContext<'a>, scoped: bool) -> Vec<Event<'a>> {
    let mut out = Vec::new();
    let marks = file.hooks.reference_marks();
    let Some(&first) = marks.first() else {
        return out;
    };
    let g = file.grammar;
    for at in memchr::memchr2_iter(first, *marks.get(1).unwrap_or(&first), file.text.as_bytes()) {
        let Some(token) = file
            .root
            .token_at_offset(TextSize::new(at as u32))
            .right_biased()
        else {
            continue;
        };
        let (kind, range) = (token.kind(), token.text_range());
        let Some(anchor) = file.hooks.reference(kind) else {
            continue;
        };
        if usize::from(range.start()) != at {
            continue;
        }
        out.push(Event {
            anchor,
            name: &file.text[at + 1..usize::from(range.end())],
            range,
            scope: if scoped {
                token.parent().map_or(range, |n| n.text_range())
            } else {
                range
            },
            document: token
                .parent_ancestors()
                .find(|n| g.has_tag(n.kind(), "document"))
                .map(|n| n.text_range().start()),
        });
    }
    out
}

rule! {
    pub NoUndefinedAlias {
        id: "no-undefined-alias",
        category: Correctness,
        default: Error,
        description: "An alias names an anchor that is not defined earlier in the same document.",
        example: "a: *nope   # no `&nope` before it",
    }

    fn check_file(&self, file: &FileContext<'_>, ctx: &mut RuleContext<'_>) {
        let mut anchors: HashSet<&str> = HashSet::new();
        let mut document = None;
        for e in events(file, false) {
            if e.document != document {
                document = e.document;
                anchors.clear();
            }
            if e.anchor {
                anchors.insert(e.name);
            } else if !anchors.contains(e.name) {
                ctx.report(e.range, format!("undefined alias `{}`", e.name));
            }
        }
    }
}

rule! {
    pub NoDuplicateAnchors {
        id: "no-duplicate-anchors",
        category: Suspicious,
        default: Warn,
        description: "An anchor name is defined again in the same document; an alias refers to the \
                  latest definition above it, which is easy to misread.",
        example: "- &a 1\n- &a 2   # reported at the second `&a`",
    }

    fn check_file(&self, file: &FileContext<'_>, ctx: &mut RuleContext<'_>) {
        let mut first = Repeats::default();
        let mut document = None;
        for e in events(file, false).into_iter().filter(|e| e.anchor) {
            if e.document != document {
                document = e.document;
                first = Repeats::default();
            }
            let message = || format!("duplicate anchor `{}`", e.name);
            first.check(ctx, e.name, e.range, message, "first defined here");
        }
    }
}

rule! {
    pub NoUnusedAnchors {
        id: "no-unused-anchors",
        category: Cleanup,
        default: Off,
        fix: Safe,
        description: "An anchor that no alias refers to. The fix removes the anchor, which changes \
                  nothing about the data.",
        example: "a: &x 1   # becomes a: 1 when no `*x` follows",
    }

    fn check_file(&self, file: &FileContext<'_>, ctx: &mut RuleContext<'_>) {
        let events = events(file, false);
        let mut used = vec![false; events.len()];
        // An alias refers to the latest anchor of its name above it.
        let mut latest: HashMap<&str, usize> = HashMap::new();
        let mut document = None;
        for (i, e) in events.iter().enumerate() {
            if e.document != document {
                document = e.document;
                latest.clear();
            }
            if e.anchor {
                latest.insert(e.name, i);
            } else if let Some(&anchor) = latest.get(e.name) {
                used[anchor] = true;
            }
        }
        for (e, _) in events.iter().zip(&used).filter(|(e, u)| e.anchor && !**u) {
            let edit = Edit::delete(removal(file.text, e.range));
            ctx.report_fix(
                e.range,
                format!("unused anchor `{}`", e.name),
                ("Remove the anchor", vec![edit]),
            );
        }
    }
}

/// The anchor and the blanks that separated it from its neighbour: the ones after it, or, at
/// the end of a line, the ones before it.
fn removal(text: &str, anchor: TextRange) -> TextRange {
    let blank = |c: char| c == ' ' || c == '\t';
    let (start, end) = (usize::from(anchor.start()), usize::from(anchor.end()));
    let after = text[end..].len() - text[end..].trim_start_matches(blank).len();
    let (from, to) = if text[end + after..].starts_with(['\n', '\r']) || end + after == text.len() {
        let before = text[..start].len() - text[..start].trim_end_matches(blank).len();
        (start - before, end)
    } else {
        (start, end + after)
    };
    TextRange::new(TextSize::new(from as u32), TextSize::new(to as u32))
}

rule! {
    pub NoRecursiveAlias {
        id: "no-recursive-alias",
        category: Correctness,
        default: Error,
        description: "An alias inside the node its own anchor is on, which makes the data refer to \
                  itself.",
        example: "a: &x\n  b: *x   # reported",
    }

    fn check_file(&self, file: &FileContext<'_>, ctx: &mut RuleContext<'_>) {
        let mut latest: HashMap<&str, TextRange> = HashMap::new();
        let mut document = None;
        for e in events(file, true) {
            if e.document != document {
                document = e.document;
                latest.clear();
            }
            if e.anchor {
                latest.insert(e.name, e.scope);
            } else if latest.get(e.name).is_some_and(|scope| scope.contains_range(e.range)) {
                ctx.report(e.range, format!("recursive alias `{}`", e.name));
            }
        }
    }
}

rule! {
    pub NoMergeKeys {
        id: "no-merge-keys",
        category: Suspicious,
        default: Off,
        description: "The merge key (YAML `<<`), which YAML 1.2 dropped and some tools do not support.",
        example: "a:\n  <<: *base   # reported",
    }

    fn check_file(&self, file: &FileContext<'_>, ctx: &mut RuleContext<'_>) {
        let g = file.grammar;
        let Some(merge) = g.keys().merge.as_deref() else {
            return;
        };
        for at in memchr::memmem::find_iter(file.text.as_bytes(), merge.as_bytes()) {
            let Some(token) = file.root.token_at_offset(TextSize::new(at as u32)).right_biased() else {
                continue;
            };
            let range = token.text_range();
            let is_key = token
                .parent()
                .and_then(|n| n.parent())
                .and_then(|kv| g.slot(&kv, "key"))
                .and_then(|k| k.into_node())
                .is_some_and(|k| token.parent().is_some_and(|n| n == k));
            if usize::from(range.start()) == at && &file.text[range] == merge && is_key {
                ctx.report(range, format!("the `{merge}` merge key is not supported"));
            }
        }
    }
}
