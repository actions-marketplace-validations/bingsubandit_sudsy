//! Rules about the `<value>`, `<param>`, `<returns>` and `<typeparam>` elements (SA1610,
//! SA1613-SA1617, SA1619, SA1621, SA1622).
//!
//! concept: new no-returns-on-void, new doc-partial-typeparams
//! what: no-returns-on-void: A `<returns>` on a method or delegate that returns `void`
//! what: doc-partial-typeparams: A documented partial generic type (with a `<summary>`) that has no `<typeparam>` for one of its type parameters

use super::doc_comments::{DOC_KINDS, Decl, documented};
use core_rules::rule;
use core_rules::{Edit, RuleContext};
use core_syntax::{SyntaxNode, TextRange};

rule! {
    pub NoReturnsOnVoid {
        id: "no-returns-on-void",
        category: Style,
        default: Off,
        fix: Safe,
        description: "Reports a `<returns>` on a method or delegate that returns `void`. The fix removes \
                  the element with its line when it is alone on it.",
        why: "There is nothing to return, so the tag documents a value that does not exist and is \
                  shown in the reference as a result.",
        example: "/// <returns>Nothing.</returns>   // remove it\npublic void Run() { }",
        kinds: DOC_KINDS,
    }

    fn check(&self, node: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        let Some(d) = documented(node, ctx, Decl::has_return_type) else {
            return;
        };
        if !d.returns_void(ctx.file.grammar) {
            return;
        }
        if let Some(returns) = d.doc.first("returns") {
            let range = returns.range();
            let removal = line_range(ctx.file.text, range);
            ctx.report_fix(
                range,
                "a void member should not document a return value",
                ("Remove the <returns> element", vec![Edit::delete(removal)]),
            );
        }
    }
}

/// The element's range, widened to the whole line when only `///` and white space share it.
fn line_range(text: &str, element: TextRange) -> TextRange {
    let (start, end) = (usize::from(element.start()), usize::from(element.end()));
    let line_start = text[..start].rfind('\n').map_or(0, |i| i + 1);
    let line_end = text[end..].find('\n').map_or(text.len(), |i| end + i + 1);
    let before = text[line_start..start].trim();
    let after = text[end..line_end].trim();
    if before == "///" && after.is_empty() {
        TextRange::new((line_start as u32).into(), (line_end as u32).into())
    } else {
        element
    }
}

rule! {
    pub DocPartialTypeparams {
        id: "doc-partial-typeparams",
        category: Style,
        default: Off,
        description: "Reports a documented partial generic type (one that has a `<summary>`) that has no \
                  `<typeparam>` for one of its type parameters, on the parameter. Each part is checked \
                  on its own. No fix.",
        why: "Without the tag a reader of the documentation sees `T` and has to read the code to \
                  learn what it stands for.\n\
                  To fix, add `<typeparam name=\"T\">` with the meaning of `T`.",
        example: "/// <summary>Box.</summary>\npublic partial class Box<T> { }   // add <typeparam name=\"T\">",
        kinds: DOC_KINDS,
    }

    fn check(&self, node: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        let Some(d) = documented(node, ctx, Decl::is_type) else {
            return;
        };
        if !d.is_partial() || !d.needs_comment() || d.doc.has_inheritdoc() {
            return;
        }
        if d.doc.first("summary").is_none() {
            return;
        }
        let documented_names: Vec<String> = d
            .doc
            .all("typeparam")
            .filter_map(|e| e.declared_name())
            .map(|a| a.value)
            .collect();
        for parameter in d.type_parameters() {
            if !documented_names.iter().any(|n| n == parameter.text()) {
                ctx.report(
                    parameter.text_range(),
                    format!("the type parameter '{}' should be documented", parameter.text()),
                );
            }
        }
    }
}
