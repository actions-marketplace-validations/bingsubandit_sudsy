//! The `<summary>` rules (SA1601, SA1604-SA1608). A partial type or method is checked part by
//! part: StyleCop accepts one documented part, which needs the whole project.
//!
//! concept: new doc-partial-required, new no-default-summary
//! what: doc-partial-required: A partial type or method without a documentation comment (or with an empty one)
//! what: no-default-summary: A `<summary>` that starts with the text Visual Studio generates, "Summary description for ..."

use super::doc_comments::{DOC_KINDS, Decl, Declaration, doc_of, documented, normalized_text};
use super::kind;
use core_rules::RuleContext;
use core_rules::rule;
use core_syntax::SyntaxNode;

const DEFAULT_SUMMARY: &str = "Summary description for";

rule! {
    pub DocPartialRequired {
        id: "doc-partial-required",
        category: Style,
        default: Off,
        description: "Reports a partial type or method without a documentation comment, or with an empty \
                  one. Each part is checked on its own: accepting one documented part would need all \
                  the other parts of the project. Which elements need documentation follows the \
                  default of private members being left out.",
        why: "A partial type is built from several files, and the documentation tools take the \
                  comment from one of them. A part without a comment leaves the reader of that file \
                  with nothing to read at the declaration.\n\
                  To fix, document the part.",
        example: "public partial class A { }   // add a /// <summary> comment",
        kinds: &[
            kind::CLASS_DECLARATION,
            kind::STRUCT_DECLARATION,
            kind::INTERFACE_DECLARATION,
            kind::RECORD_DECLARATION,
            kind::RECORD_STRUCT_DECLARATION,
            kind::METHOD_DECLARATION,
        ],
    }

    fn check(&self, node: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        let Some(kind) = Decl::of(node.kind()) else {
            return;
        };
        let decl = Declaration {
            node: node.clone(),
            kind,
        };
        if !decl.is_partial_element() || !decl.needs_comment() {
            return;
        }
        if doc_of(node, ctx.file.grammar).is_some_and(|d| !d.is_blank()) {
            return;
        }
        for range in decl.names() {
            ctx.report(range, "partial elements should be documented");
        }
    }
}

rule! {
    pub NoDefaultSummary {
        id: "no-default-summary",
        category: Style,
        default: Off,
        description: "Reports a `<summary>` that starts with the text Visual Studio generates, \"Summary \
                  description for ...\". A comment with `<inheritdoc />` is not reported. No fix.",
        why: "The text is a placeholder that the tool inserts. It stays in the generated \
                  documentation as a sentence that says nothing about the element.\n\
                  To fix, write the real summary.",
        example: "/// <summary>Summary description for Foo</summary>   // say what Foo is\nclass Foo { }",
        kinds: DOC_KINDS,
    }

    fn check(&self, node: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        let Some(d) = documented(node, ctx, Decl::is_element) else {
            return;
        };
        if d.doc.has_inheritdoc() {
            return;
        }
        for summary in d.doc.all("summary") {
            let first = summary.content().into_iter().next();
            if let Some(text) = first.filter(|c| c.kind() == kind::XML_TEXT)
                && normalized_text(&text)
                    .trim_start()
                    .starts_with(DEFAULT_SUMMARY)
                {
                    ctx.report(summary.range(), "the <summary> is the default text");
                    return;
                }
        }
    }
}
