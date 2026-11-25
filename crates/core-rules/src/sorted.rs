//! Ordering rules on the shared tags: `sorted-keys` (every `#mapping`) and `sorted-array-values`
//! (every `#sequence` of only strings or only numbers).
//!
//! concept: sorted-keys, sorted-array-values
//! what: Mapping keys and string or number array values in order.
//! rules: sorted-keys, sorted-array-values
//! reads: rule:sorted-keys

use crate::hooks::Scalar;
use crate::keys::{key_range, keyed};
use crate::options::{Check, Opt, OptionDecl};
use crate::paths::{self, PATH_PATTERNS};
use crate::rule;
use crate::rule::RuleContext;
use crate::sorting::{self, Sorting};
use crate::tree::{has_separator, reorder_edits, reorder_range, significant_range};
use core_syntax::{SyntaxElement, SyntaxNode, TextRange};
use std::cmp::Ordering;

rule! {
    pub SortedKeys {
        id: "sorted-keys",
        category: Style,
        default: Off,
        fix: Unsafe,
        description: "Reports the keys of a mapping that are not in order (alphabetical by default). By \
              default the fewest keys that would have to move are reported (`report: moved`), \
              `misplaced` reports every key that sorts before one above it. The first report \
              carries the fix. Comments on lines of their own stay where they are and a comment \
              after an entry moves with it. Options: `order` (keys that come first), \
              `direction`, `compare` (`ordinal`, `case_insensitive`, `natural`), `ignored_keys`, \
              `min_keys` (default 2), `blank_line_groups`, `path_patterns` and `has_properties` \
              (only mappings that have all of these keys). The fix is unsafe: moving a YAML \
              anchor below its alias breaks the document.",
        why: "Keys in a fixed order are quick to look up, and two branches that add different \
              keys to a sorted file rarely conflict. The order of a mapping is usually not data, \
              but where a tool reads it (a build order, a dependency list that is applied top \
              down) the rule should be limited with `path_patterns` or turned off.\n\
              To fix, sort the keys; the fix does it.",
        example: "{ \"b\": 1, \"a\": 2 }   // becomes { \"a\": 2, \"b\": 1 }",
        options: [
            sorting::ORDER,
            sorting::DIRECTION,
            sorting::COMPARE,
            sorting::IGNORED_KEYS,
            sorting::MIN_KEYS,
            sorting::BLANK_LINE_GROUPS,
            PATH_PATTERNS,
            OptionDecl {
                name: "has_properties",
                default: Opt::List(&[]),
                check: Check::Any,
                description: "Only mappings that have all of these keys are checked.",
            },
            OptionDecl {
                name: "report",
                default: Opt::Str("moved"),
                check: Check::OneOf(&["moved", "misplaced"]),
                description: "Which keys are reported. `moved`: the fewest keys that would have to \
                          move (eslint-plugin-jsonc). `misplaced`: every key that sorts before \
                          one above it (yamllint).",
            },
    ],
        tags: &["mapping"],
    }

    fn check(&self, node: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        let (grammar, hooks, text) = (ctx.file.grammar, ctx.file.hooks, ctx.file.text);
        let entries = keyed(node, grammar);
        let names: Vec<_> = entries
            .iter()
            .map(|(_, k)| hooks.key_name(grammar, k, text))
            .collect();
        if !ctx
            .options
            .list("has_properties")
            .iter()
            .all(|p| names.iter().any(|n| n == p))
            || !paths::in_scope(node, ctx)
        {
            return;
        }
        let separated = has_separator(node, grammar);
        let range = |i: usize| reorder_range(&entries[i].0, separated, grammar);
        let sorting = Sorting::new(ctx.options, "min_keys");
        let blank = |i: usize| {
            sorting::has_blank_line(
                &text[usize::from(range(i - 1).end())..usize::from(range(i).start())],
            )
        };
        let Some(plan) = sorting.plan_keys(&names, blank) else {
            return;
        };
        let ranges: Vec<TextRange> = (0..entries.len()).map(range).collect();
        let edits = reorder_edits(text, &ranges, &plan.sorted);
        let mut fix = Some(("Sort the keys of this mapping", edits));
        let reported = if ctx.options.string("report") == "misplaced" {
            &plan.misplaced
        } else {
            &plan.moved
        };
        for &i in reported {
            ctx.report_fix(key_range(&entries[i].1, grammar), "keys are not sorted", fix.take());
        }
    }
}

rule! {
    pub SortedArrayValues {
        id: "sorted-array-values",
        category: Style,
        default: Off,
        fix: Unsafe,
        description: "Reports the items of a sequence that are not in order, for sequences of only \
              strings or only numbers (ascending by default; numbers compare as numbers). A \
              sequence of other or mixed items is skipped. The fewest items that would have to \
              move are reported, and the first report carries the fix. The fix is unsafe: the \
              order of a sequence can be data. Options: `direction`, `compare` (`ordinal`, \
              `case_insensitive`, `natural`), `min_values` (default 2), `blank_line_groups` and \
              `path_patterns` (only the sequences whose key path matches one).",
        why: "A list that is kept sorted is easy to scan and to merge: new entries go to one \
              place, and two branches that add different entries rarely conflict. The rule only \
              makes sense where the order is not data, so it is off by default.\n\
              To fix, sort the items, or turn the rule off for that list with `path_patterns`.",
        example: "[\"b\", \"a\"]   // becomes [\"a\", \"b\"]",
        options: [
            sorting::DIRECTION,
            sorting::COMPARE,
            OptionDecl {
                name: "min_values",
                default: Opt::Int(2),
                check: Check::Any,
                description: "Sequences with fewer items are not checked.",
            },
            sorting::BLANK_LINE_GROUPS,
            PATH_PATTERNS,
    ],
        tags: &["sequence"],
    }

    fn check(&self, node: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        let (grammar, hooks, text) = (ctx.file.grammar, ctx.file.hooks, ctx.file.text);
        if !paths::in_scope(node, ctx) {
            return;
        }
        // `entry` is what moves (a YAML entry with its dash), `item` the value node in it.
        let separated = has_separator(node, grammar);
        let mut entries: Vec<SyntaxNode> = Vec::new();
        let mut scalars: Vec<Scalar> = Vec::new();
        let mut at: Vec<TextRange> = Vec::new();
        for entry in node.children() {
            let item = if grammar.has_tag(entry.kind(), "value") {
                entry.clone()
            } else if let Some(SyntaxElement::Node(value)) = grammar.slot(&entry, "value") {
                value
            } else {
                return;
            };
            let Some(scalar) = hooks.scalar(grammar, &item, text) else {
                return;
            };
            at.push(significant_range(&item, grammar));
            scalars.push(scalar);
            entries.push(entry);
        }
        let sorting = Sorting::new(ctx.options, "min_values");
        let blank = |i: usize| {
            sorting::has_blank_line(&text[usize::from(at[i - 1].end())..usize::from(at[i].start())])
        };
        let plan = match scalars.first() {
            Some(Scalar::Number(_)) => {
                let numbers: Option<Vec<f64>> = scalars
                    .iter()
                    .map(|s| match s {
                        Scalar::Number(n) => Some(*n),
                        Scalar::String(_) => None,
                    })
                    .collect();
                numbers.and_then(|n| {
                    let order = |a: usize, b: usize| {
                        sorting.directed(n[a].partial_cmp(&n[b]).unwrap_or(Ordering::Equal))
                    };
                    sorting.plan(n.len(), blank, |_| false, order)
                })
            }
            Some(Scalar::String(_)) => {
                let strings: Option<Vec<_>> = scalars
                    .iter()
                    .map(|s| match s {
                        Scalar::String(v) => Some(v.as_ref()),
                        Scalar::Number(_) => None,
                    })
                    .collect();
                strings.and_then(|s| sorting.plan_keys(&s, blank))
            }
            None => None,
        };
        let Some(plan) = plan else {
            return;
        };
        let ranges: Vec<TextRange> = entries
            .iter()
            .map(|e| reorder_range(e, separated, grammar))
            .collect();
        let edits = reorder_edits(text, &ranges, &plan.sorted);
        let mut fix = Some(("Sort the values of this sequence", edits));
        for &i in &plan.moved {
            ctx.report_fix(at[i], "values are not sorted", fix.take());
        }
    }
}
