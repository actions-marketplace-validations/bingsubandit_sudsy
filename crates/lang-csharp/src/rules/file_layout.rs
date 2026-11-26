//! Rules about what a file holds: `single-class-per-file`, `single-namespace-per-file` and
//! `file-name-matches-type`. They look at the file's top level only.
//!
//! concept: new single-class-per-file, new single-namespace-per-file, new file-name-matches-type
//! what: single-class-per-file: A file holds one class or record: every later one is reported, more parts of the first class (when it is partial) excepted
//! what: single-namespace-per-file: A file declares one namespace: every later one, nested ones included, is reported
//! what: file-name-matches-type: The file name, up to its first dot and ignoring case, is the name of the first type in the file; a generic type may also be written `Name{T,U}`

use super::{kind, text_of};
use core_rules::RuleContext;
use core_rules::rule;
use core_rules::tree::significant_range;
use core_syntax::{SyntaxKind, SyntaxNode, SyntaxNodeExt, SyntaxToken};

const CLASSES: &[SyntaxKind] = &[kind::CLASS_DECLARATION, kind::RECORD_DECLARATION];

const TYPES: &[SyntaxKind] = &[
    kind::CLASS_DECLARATION,
    kind::RECORD_DECLARATION,
    kind::STRUCT_DECLARATION,
    kind::RECORD_STRUCT_DECLARATION,
    kind::INTERFACE_DECLARATION,
    kind::ENUM_DECLARATION,
    kind::DELEGATE_DECLARATION,
];

/// The type declarations and namespaces that are not inside another type, in source order.
fn collect_top_level(
    node: &SyntaxNode,
    types: &mut Vec<SyntaxNode>,
    namespaces: &mut Vec<SyntaxNode>,
) {
    for child in node.children() {
        let kind = child.kind();
        if kind == kind::NAMESPACE_DECLARATION || kind == kind::FILE_SCOPED_NAMESPACE_DECLARATION {
            namespaces.push(child.clone());
            collect_top_level(&child, types, namespaces);
        } else if TYPES.contains(&kind) {
            types.push(child);
        }
    }
}

fn top_level(root: &SyntaxNode) -> (Vec<SyntaxNode>, Vec<SyntaxNode>) {
    let (mut types, mut namespaces) = (Vec::new(), Vec::new());
    collect_top_level(root, &mut types, &mut namespaces);
    (types, namespaces)
}

fn name_token(declaration: &SyntaxNode) -> Option<SyntaxToken> {
    declaration.token(kind::IDENTIFIER_TOKEN)
}

fn is_partial(declaration: &SyntaxNode) -> bool {
    declaration
        .children_with_tokens()
        .any(|e| e.kind() == kind::PARTIAL_KEYWORD)
}

rule! {
    pub SingleClassPerFile {
        id: "single-class-per-file",
        category: Style,
        default: Off,
        description: "Reports a class or record that comes after the first one in a file: more parts of \
                  the first class (when it is partial) are not reported. Structs, interfaces, enums \
                  and delegates are not counted. No fix.",
        why: "A file with one class is found by the name of the class, and a change to a class \
                  touches one file. A file with several is a place where a class has to be searched \
                  for.\n\
                  To fix, move the class into a file of its own.",
        example: "class A { } class B { }   // move B to its own file",
        kinds: &[kind::COMPILATION_UNIT],
    }

    fn check(&self, node: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        let mut first: Option<String> = None;
        for class in top_level(node)
            .0
            .iter()
            .filter(|t| CLASSES.contains(&t.kind()))
        {
            let Some(name) = name_token(class) else {
                continue;
            };
            if is_partial(class) && first.as_deref() == Some(name.text()) {
                continue;
            }
            if first.is_some() {
                ctx.report(name.text_range(), "a file may only contain a single class");
            }
            first.get_or_insert_with(|| name.text().to_string());
        }
    }
}

rule! {
    pub SingleNamespacePerFile {
        id: "single-namespace-per-file",
        category: Style,
        default: Off,
        description: "Reports a namespace that comes after the first one in a file; nested namespaces are \
                  reported too. No fix.",
        why: "A file with one namespace has one place where its names come from. Several \
                  namespaces in a file make the meaning of a name depend on where in the file it is \
                  read.\n\
                  To fix, move the later namespaces into files of their own.",
        example: "namespace A { } namespace B { }   // move B to its own file",
        kinds: &[kind::COMPILATION_UNIT],
    }

    fn check(&self, node: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        for namespace in top_level(node).1.iter().skip(1) {
            if let Some(name) = namespace.children().next() {
                ctx.report(
                    significant_range(&name, ctx.file.grammar),
                    "a file may only contain a single namespace",
                );
            }
        }
    }
}

rule! {
    pub FileNameMatchesType {
        id: "file-name-matches-type",
        category: Style,
        default: Off,
        description: "Reports a file whose name, up to its first dot and ignoring case, is not the name of \
                  the first type in the file; a generic type may also be written `Name{T,U}`. Not \
                  checked when that type is partial. No fix.",
        why: "A file named after its type is found by the name from the file tree, from a stack \
                  trace and from a search. A file whose name says something else sends the reader to \
                  the wrong place.\n\
                  To fix, rename the file or the type.",
        example: "// Orders.cs\nclass Order { }   // rename the file Order.cs",
        kinds: &[kind::COMPILATION_UNIT],
    }

    fn check(&self, node: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        let Some(first) = top_level(node).0.into_iter().next() else {
            return;
        };
        let (Some(name), false) = (name_token(&first), is_partial(&first)) else {
            return;
        };
        let Some(file) = ctx.file.path.file_name().and_then(|f| f.to_str()) else {
            return;
        };
        let stem = file.split('.').next().unwrap_or(file);
        let generic = first
            .child(kind::TYPE_PARAMETER_LIST)
            .map(|list| {
                let names: Vec<&str> = list.children().map(|p| text_of(&p, ctx)).collect();
                format!("{}{{{}}}", name.text(), names.join(","))
            });
        let matches = stem.eq_ignore_ascii_case(name.text())
            || generic.is_some_and(|g| stem.eq_ignore_ascii_case(&g));
        if !matches {
            ctx.report(
                name.text_range(),
                "the file name should match the first type in it",
            );
        }
    }
}
