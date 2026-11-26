//! `block-indentation`: a block collection that starts on the line after its key (or its dash) is
//! indented by the same number of columns everywhere in the file, and a block sequence under a key
//! is indented. The columns are those of the first token of the collection and of the key (the
//! dash) it belongs to, read from the `#key-value` entries and `#sequence` entries of block
//! collections (`#block`).
//!
//! concept: block-indentation
//! what: Block collections under a key or a dash indented by one width per file; block sequences under a key indented.
//! tools: yamllint indentation
//! rules: block-indentation
//! reads: tag:block, tag:key-value, tag:sequence

use crate::options::{Check, Opt, OptionDecl};
use crate::rule;
use crate::rule::{FileContext, RuleContext};
use crate::tree::significant_range;
use core_syntax::{SyntaxElement, TextRange, TextSize};

rule! {
    pub BlockIndentation {
        id: "block-indentation",
        category: Style,
        default: Off,
        description: "Reports a block collection or scalar that starts on the line after its key \
              (or its dash) and is not indented by `spaces` columns more than the key. With `spaces` \
              0 the width is the one of the first such indentation in the file. With \
              `indent_sequences` a block sequence under a key must be indented too: dashes at the \
              column of the key are reported. Flow collections and keys with a value on the same \
              line are not checked.",
        why: "A file that indents one block by two columns and the next by four is harder to read, \
              and dashes at the column of their key make a list look like a sibling of the key. \
              A single width keeps the shape of the data visible and the diffs small.\n\
              To fix, indent the block by the width the file uses; the formatter does it.",
        example: "items:\n- a   # reported: indent the dash under `items`",
        options: [
            OptionDecl {
                name: "spaces",
                default: Opt::Int(0),
                check: Check::Any,
                description: "Columns of one indentation level; 0 takes the first one found in the file.",
            },
            OptionDecl {
                name: "indent_sequences",
                default: Opt::Bool(true),
                check: Check::Any,
                description: "A block sequence under a key must be indented, not at the key's column.",
            },
        ],
    }

    fn check_file(&self, file: &FileContext<'_>, ctx: &mut RuleContext<'_>) {
        let g = file.grammar;
        let mut spaces = usize::try_from(ctx.options.int("spaces"))
            .ok()
            .filter(|s| *s > 0);
        let indent_sequences = ctx.options.bool("indent_sequences");
        let col = |at: TextSize| {
            let (line, col) = file.lines.line_col(file.text, at);
            (line, col - 1)
        };
        for node in file.root.descendants() {
            let Some(parent) = node.parent() else { continue };
            if !g.has_tag(parent.kind(), "block") {
                continue;
            }
            let is_entry = g.has_tag(parent.kind(), "mapping");
            let (Some(key), Some(SyntaxElement::Node(value))) = (
                if is_entry { g.slot(&node, "key") } else { Some(SyntaxElement::Node(node.clone())) },
                g.slot(&node, "value"),
            ) else {
                continue;
            };
            if g.name(value.kind()) == "PropertyNode" {
                continue;
            }
            let base_at = match &key {
                SyntaxElement::Node(n) => significant_range(n, g).start(),
                SyntaxElement::Token(t) => t.text_range().start(),
            };
            let at = significant_range(&value, g).start();
            let ((key_line, key_col), (line, found)) = (col(base_at), col(at));
            if line == key_line {
                continue;
            }
            let flush = is_entry && found == key_col && g.has_tag(value.kind(), "sequence");
            let message = if flush && indent_sequences {
                match spaces {
                    Some(s) => format!("wrong indentation: expected {} but found {found}", key_col + s),
                    None => format!("wrong indentation: expected at least {}", found + 1),
                }
            } else if flush {
                continue;
            } else {
                let width = *spaces.get_or_insert(found.saturating_sub(key_col));
                if found == key_col + width {
                    continue;
                }
                format!("wrong indentation: expected {} but found {found}", key_col + width)
            };
            ctx.report(TextRange::at(at, TextSize::from(1)), message);
        }
    }
}
