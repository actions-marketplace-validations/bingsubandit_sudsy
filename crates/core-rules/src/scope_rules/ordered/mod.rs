//! The scope rules of a language where the order of the code decides what a name means (Python,
//! after pyflakes and ruff's `F` rules): a read finds the binding before it, a rebinding replaces
//! the old one unless something read it, and branches of one `if` or `try` do not replace each
//! other. The language says which nodes are branches, loops, collections and the like by tags
//! (`#branch`, `#branching`, `#collection`, `#comprehension`, `#with-item`, `#named-expr`,
//! docs/ai/rules/semantics.md#ordered-rules) and the rest in [`Ordered`](super::Ordered).

mod defs;
mod imports;
mod live;
mod names;
pub(super) mod quoted;

use super::cx::Cx;
use crate::FileContext;
use crate::checks::FileCache;
use core_syntax::TextRange;
use defs::Local;
use live::{Bindings, Lives};
use std::rc::Rc;

pub(super) type Hit = (TextRange, String);

/// What the name an import declaration binds stands for (`a` for `import a.b`, `m.a` for
/// `from m import a`).
pub(super) fn import_target(
    g: &core_syntax::Grammar,
    root: &core_syntax::SyntaxNode,
    pragma: &str,
    d: &core_semantics::scopes::Decl,
) -> Option<String> {
    imports::describe(g, root, pragma, d).map(|i| i.target)
}

/// What the ordered rules find in a file, worked out once for all of them from the model.
#[derive(Default)]
pub(super) struct Findings {
    unused_import: Vec<Hit>,
    redeclared: Vec<Hit>,
    shadowed: Vec<Hit>,
    undefined: Vec<Hit>,
    star_usage: Vec<Hit>,
    undefined_export: Vec<Hit>,
    use_before_define: Vec<Hit>,
    unused_variable: Vec<Hit>,
    unused_annotation: Vec<Hit>,
    unused_unpacked: Vec<Hit>,
}

/// What a rule reports of the findings.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Which {
    UnusedImport,
    Redeclared,
    Shadowed,
    Undefined,
    StarUsage,
    UndefinedExport,
    UseBeforeDefine,
    UnusedVariable,
    UnusedAnnotation,
    UnusedUnpacked,
}

impl Findings {
    pub fn get(&self, which: Which) -> &[Hit] {
        match which {
            Which::UnusedImport => &self.unused_import,
            Which::Redeclared => &self.redeclared,
            Which::Shadowed => &self.shadowed,
            Which::Undefined => &self.undefined,
            Which::StarUsage => &self.star_usage,
            Which::UndefinedExport => &self.undefined_export,
            Which::UseBeforeDefine => &self.use_before_define,
            Which::UnusedVariable => &self.unused_variable,
            Which::UnusedAnnotation => &self.unused_annotation,
            Which::UnusedUnpacked => &self.unused_unpacked,
        }
    }
}

thread_local! {
    static FINDINGS: FileCache<Findings> = const { FileCache::new() };
}

fn sorted(mut hits: Vec<Hit>) -> Vec<Hit> {
    hits.sort_by_key(|(r, _)| r.start());
    hits.dedup();
    hits
}

/// The findings of the file, worked out by the first rule that asks.
pub(super) fn findings(cx: &Cx, file: &FileContext<'_>) -> Rc<Findings> {
    FINDINGS.with(|cache| cache.get(file, || compute(cx)))
}

fn compute(cx: &Cx) -> Findings {
    let bs = Bindings::new(cx);
    let lv = Lives::new(cx, &bs);
    let exported = names::exports(cx);
    let (undefined, mut star_usage) = names::unresolved(cx, &bs);
    let undefined_export = names::export_findings(cx, &exported, &mut star_usage);
    let (redefined, mut loops) = defs::redefinitions(cx, &lv);
    loops.extend(defs::redefined_arguments(cx, &bs));
    Findings {
        unused_import: sorted(defs::unused_imports(cx, &lv, &exported)),
        redeclared: sorted(redefined),
        shadowed: sorted(loops),
        undefined: sorted(undefined),
        star_usage: sorted(star_usage),
        undefined_export: sorted(undefined_export),
        use_before_define: sorted(names::use_before_define(cx, &bs)),
        unused_variable: sorted(defs::unused_locals(cx, &lv, Local::Variable)),
        unused_annotation: sorted(defs::unused_locals(cx, &lv, Local::Annotation)),
        unused_unpacked: sorted(defs::unused_locals(cx, &lv, Local::Unpacked)),
    }
}
