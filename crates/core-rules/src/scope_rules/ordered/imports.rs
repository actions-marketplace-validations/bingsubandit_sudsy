//! What an import declaration binds: the form it is written in, what a message says about it
//! and what the bound name stands for.

use core_semantics::scopes::{Decl, Flags};
use core_syntax::roles::code_text;
use core_syntax::{Grammar, SyntaxNode};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::scope_rules) enum Form {
    /// `import a`, `import a as b`.
    Import,
    /// `import a.b`: binds `a`.
    Submodule,
    /// `from m import a`.
    From,
    /// An import of the language's pragma module (`from __future__ import a`).
    Pragma,
}

pub(in crate::scope_rules) struct Import {
    pub form: Form,
    /// What a message says (`a.b`, `.m.a`).
    pub label: String,
    /// What the bound name stands for (`a` for `import a.b`).
    pub target: String,
    /// `import a as a`, `from m import a as a`: a re-export.
    pub explicit: bool,
}

/// The number of dots a relative import starts with.
fn level(g: &Grammar, stmt: &SyntaxNode) -> usize {
    stmt.children_with_tokens()
        .filter_map(|e| e.into_token())
        .filter(|t| g.has_tag(t.kind(), "dots"))
        .map(|t| t.text().len())
        .sum()
}

/// The import an alias declaration makes (`pragma`: the language's pragma module); `None` for a
/// declaration of another kind.
pub(in crate::scope_rules) fn describe(
    g: &Grammar,
    root: &SyntaxNode,
    pragma: &str,
    d: &Decl,
) -> Option<Import> {
    if !d.flags.contains(Flags::IMPORT) {
        return None;
    }
    let alias = d.node(root);
    let slot = |n: &SyntaxNode, label: &str| g.slot(n, label)?.into_node();
    let name = code_text(&slot(&alias, "name")?);
    let asname = slot(&alias, "asname").map(|a| code_text(&a));
    let stmt = alias.parent()?;
    if !g.has_field(stmt.kind(), "module") {
        return Some(match asname {
            Some(a) => Import {
                form: Form::Import,
                explicit: a == name,
                label: name.clone(),
                target: name,
            },
            None => {
                let first = name.split('.').next().unwrap_or(&name).to_owned();
                let form = if first.len() < name.len() {
                    Form::Submodule
                } else {
                    Form::Import
                };
                Import {
                    form,
                    label: name,
                    target: first,
                    explicit: false,
                }
            }
        });
    }
    let module = slot(&stmt, "module")
        .map(|m| code_text(&m))
        .unwrap_or_default();
    let level = level(g, &stmt);
    let dots = ".".repeat(level);
    let full = if module.is_empty() {
        format!("{dots}{name}")
    } else {
        format!("{dots}{module}.{name}")
    };
    Some(Import {
        form: if level == 0 && !pragma.is_empty() && module == pragma {
            Form::Pragma
        } else {
            Form::From
        },
        explicit: asname.is_some_and(|a| a == name),
        label: full.clone(),
        target: full,
    })
}
