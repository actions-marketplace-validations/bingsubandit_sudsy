//! What a declaration is where the order of the code matters: the kind of binding, where it takes
//! effect, which branch it is in, and whether anything read it. The model resolves by name; the
//! order (a read between two assignments) is read off the positions here.

use super::imports::{self, Form, Import};
use crate::scope_rules::cx::Cx;
use core_semantics::scopes::{Decl, Flags, ScopeId, SymbolId};
use core_syntax::{SyntaxNode, TextRange, TextSize};
use std::cell::OnceCell;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Kind {
    Import(Form),
    FunctionDef,
    ClassDef,
    Assignment,
    NamedExpr,
    /// `x: int` with no value.
    Annotation,
    Argument,
    LoopVar,
    WithItem,
    /// `except E as name`.
    Except,
    Comprehension,
    TypeParam,
    /// A name a `global` statement adds to the module.
    Global,
}

impl Kind {
    pub fn is_import(self) -> bool {
        matches!(self, Kind::Import(_))
    }
}

/// The compound statement (or clause) a node is in, within its scope: bindings of different
/// branches do not redefine each other. `None` outside any.
pub(super) type Branch = Option<(TextSize, u8)>;

pub(super) struct Binding {
    pub kind: Kind,
    /// From here on a read finds the binding (the end of the statement that makes it).
    pub effect: TextSize,
    pub branch: Branch,
    /// Part of a tuple or list target whose value is not a literal sequence.
    pub unpacked: bool,
    pub import: Option<Import>,
    /// What a finding points at: the name, or the whole dotted name of `import a.b`.
    pub range: TextRange,
}

/// The statement or clause a name of a target belongs to, through collections and wrappers;
/// whether it is inside a collection.
fn host(cx: &Cx, binding: &SyntaxNode) -> Option<(SyntaxNode, bool)> {
    let mut nested = false;
    for a in binding.ancestors().skip(1) {
        if cx.tag(&a, "collection") {
            nested = true;
        } else if !(cx.tag(&a, "transparent") || cx.tag(&a, "rest")) {
            return Some((a, nested));
        }
    }
    None
}

pub(super) fn branch(cx: &Cx, n: &SyntaxNode) -> Branch {
    let mut child = n.clone();
    for a in n.ancestors().skip(1) {
        let start = a.text_range().start();
        if cx.tag(&a, "branch") {
            return Some((start, 0));
        }
        if cx.tag(&a, "branching") && cx.tag(&child, "statement") {
            let after_finally = cx.tag(&a, "try")
                && a.children_with_tokens()
                    .filter_map(|e| e.into_token())
                    .find(|t| cx.file.grammar.has_tag(t.kind(), "finally"))
                    .is_some_and(|f| child.text_range().start() >= f.text_range().end());
            return Some((start, if after_finally { 2 } else { 0 }));
        }
        if cx.tag(&a, "scope") {
            return None;
        }
        child = a;
    }
    None
}

/// Describes a declaration.
pub(super) fn binding(cx: &Cx, d: &Decl) -> Binding {
    let node = cx.decl_node(d);
    let f = d.flags;
    let mut b = Binding {
        kind: Kind::Assignment,
        effect: d.name.start(),
        branch: branch(
            cx,
            &if f.contains(Flags::FUNCTION) || f.contains(Flags::CLASS) {
                node.clone()
            } else {
                cx.binding(d)
            },
        ),
        unpacked: false,
        import: None,
        range: d.name,
    };
    if f.contains(Flags::IMPORT) {
        b.import = imports::describe(cx.file.grammar, cx.root(), cx.ordered().pragma, d);
        b.kind = Kind::Import(b.import.as_ref().map_or(Form::Import, |i| i.form));
        b.effect = node.text_range().end();
        if b.kind == Kind::Import(Form::Submodule)
            && let Some(name) = cx.slot(&node, "name")
        {
            b.range = cx.range(&name);
        }
    } else if f.contains(Flags::FUNCTION) {
        b.kind = Kind::FunctionDef;
    } else if f.contains(Flags::CLASS) {
        b.kind = Kind::ClassDef;
    } else if f.contains(Flags::PARAMETER) {
        b.kind = Kind::Argument;
    } else if f.contains(Flags::CATCH) {
        b.kind = Kind::Except;
    } else if f.contains(Flags::TYPE_PARAMETER) {
        b.kind = Kind::TypeParam;
    } else if f.contains(Flags::REDIRECT) {
        b.kind = Kind::Global;
    } else if cx.tag(&node, "reference") {
        let Some((h, nested)) = host(cx, &node) else {
            return b;
        };
        b.effect = h.text_range().end();
        if cx.tag(&h, "named-expr") {
            b.kind = Kind::NamedExpr;
        } else if cx.tag(&h, "assignment") {
            if f.contains(Flags::BARE) {
                b.kind = Kind::Annotation;
            } else {
                let literal = h
                    .children()
                    .last()
                    .is_some_and(|v| cx.tag(&v, "collection"));
                b.unpacked = nested && !literal;
            }
        } else if cx.tag(&h, "loop") {
            b.kind = Kind::LoopVar;
            b.unpacked = nested;
            b.effect = cx
                .slot(&h, "iterable")
                .map_or(b.effect, |i| i.text_range().end());
        } else if cx.tag(&h, "comprehension") {
            b.kind = Kind::Comprehension;
            b.unpacked = nested;
            // The element comes first in the text and reads the target.
            b.effect = h.parent().map_or(b.effect, |c| c.text_range().start());
        } else if cx.tag(&h, "with-item") {
            b.kind = Kind::WithItem;
            b.unpacked = nested;
        }
    }
    b
}

/// The bindings of the declarations of the symbols, each worked out when first asked for.
pub(super) struct Bindings<'a> {
    cx: &'a Cx<'a>,
    v: Vec<OnceCell<Vec<Binding>>>,
}

impl<'a> Bindings<'a> {
    pub fn new(cx: &'a Cx<'a>) -> Self {
        Bindings {
            cx,
            v: cx.m.symbol_ids().map(|_| OnceCell::new()).collect(),
        }
    }

    /// One for each declaration of the symbol, in order.
    pub fn get(&self, id: SymbolId) -> &[Binding] {
        self.v[id.0 as usize].get_or_init(|| {
            (self.cx.m.symbol(id).decls.iter())
                .map(|d| binding(self.cx, d))
                .collect()
        })
    }
}

/// A declaration that takes effect, with what it is and whether anything read it.
pub(super) struct Live<'a> {
    pub decl: &'a Decl,
    pub b: &'a Binding,
    /// Read, or made by a statement of another scope (`global`), or inherited from the binding it
    /// replaces: a rebinding inherits what read the old one.
    pub used: bool,
}

/// Does a function lie between the reference's scope and `scope` (the reference runs after the
/// code of `scope` has run)?
pub(super) fn deferred(cx: &Cx, from: ScopeId, scope: ScopeId) -> bool {
    cx.m.ancestors(from)
        .take_while(|s| *s != scope)
        .any(|s| cx.is_function(&cx.m.scope(s).node))
}

/// Is the declaration made by code of the symbol's own scope (a function or class name is bound
/// by the statement that opens the scope below it)?
pub(super) fn own(cx: &Cx, scope: ScopeId, d: &Decl) -> bool {
    d.from == scope || cx.m.scope(d.from).node.text_range() == d.node_range
}

/// The declarations of a symbol that take effect, in source order: an annotation without a value
/// does not replace a binding.
pub(super) fn live<'a>(cx: &'a Cx<'a>, bs: &'a Bindings<'_>, id: SymbolId) -> Vec<Live<'a>> {
    let sym = cx.m.symbol(id);
    let mut out: Vec<Live<'a>> = Vec::with_capacity(sym.decls.len());
    for (d, b) in sym.decls.iter().zip(bs.get(id)) {
        if b.kind == Kind::Annotation && !out.is_empty() {
            continue;
        }
        let used = !own(cx, sym.scope, d);
        out.push(Live { decl: d, b, used });
    }
    let mut direct = vec![false; out.len()];
    for r in cx.m.refs_of(id).filter(|r| r.is_read()) {
        // A read that runs later than the code of the scope (a function, an annotation) finds the
        // last binding; one before the first binding finds none of these.
        let i = if r.is_type() || deferred(cx, r.scope, sym.scope) {
            out.len() - 1
        } else {
            match out.iter().rposition(|l| l.b.effect <= r.range.start()) {
                Some(i) => i,
                None => continue,
            }
        };
        direct[i] = true;
    }
    let mut inherited = false;
    for (l, d) in out.iter_mut().zip(direct) {
        inherited |= d || l.used;
        l.used = inherited;
    }
    out
}

/// The live declarations of the symbols, each worked out when first asked for.
pub(super) struct Lives<'a> {
    cx: &'a Cx<'a>,
    pub bs: &'a Bindings<'a>,
    v: Vec<OnceCell<Vec<Live<'a>>>>,
}

impl<'a> Lives<'a> {
    pub fn new(cx: &'a Cx<'a>, bs: &'a Bindings<'a>) -> Self {
        Lives {
            cx,
            bs,
            v: cx.m.symbol_ids().map(|_| OnceCell::new()).collect(),
        }
    }

    pub fn get(&self, id: SymbolId) -> &[Live<'a>] {
        self.v[id.0 as usize].get_or_init(|| live(self.cx, self.bs, id))
    }
}

/// Is the name one that is meant to be unused (`_`, `__`, `_ignored`, the language's markers)?
pub(super) fn is_dummy(cx: &Cx, name: &str) -> bool {
    (name.starts_with('_')
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        && (name.bytes().all(|b| b == b'_') || name.ends_with(|c: char| c.is_ascii_alphanumeric())))
        || cx.ordered().ignored.contains(&name)
}
