//! The scope model of a file, for the rules that need to know what a name binds to: the shared
//! binder of `core_semantics::scopes` run over the tree with JavaScript's table (`scope.rows`). Built on the first scope rule that runs on a file and shared
//! with the others, so a run with syntax rules only never pays for it.

use crate::adapter::source_type;
use crate::generated::kind as k;
use core_rules::checks::FileCache;
use core_rules::tree::significant_range;
use core_rules::{FileContext, ScopeLang};
use core_semantics::scopes::{Model, SymbolId};
use core_syntax::{Grammar, SyntaxNode, SyntaxNodeExt, TextRange, grammars};
use std::cell::OnceCell;
use std::collections::HashMap;
use std::rc::Rc;

/// The scope rules' view of the language: `../scope.rows` and the globals of `../globals`.
pub static LANG: ScopeLang = ScopeLang::new(include_str!("../scope.rows"))
    .globals(include_str!("../globals/es2026.txt"))
    .globals_if(&[("commonjs", include_str!("../globals/commonjs.txt"))]);

/// The model of a script that another file embeds (a Vue `<script>`), for the host's rules.
pub fn bind_embedded(root: &SyntaxNode) -> Model {
    LANG.bind(root, grammars::JS.get())
}

/// What a scope rule reads: the model and the file's kind.
pub struct Cx {
    pub s: Rc<Model>,
    pub root: SyntaxNode,
    pub g: &'static Grammar,
    pub ts: bool,
    by_binding: OnceCell<HashMap<u32, SymbolId>>,
}

impl Cx {
    /// The child node in the body item `label` (a field of the grammar).
    pub fn slot(&self, n: &SyntaxNode, label: &str) -> Option<SyntaxNode> {
        match self.g.slot(n, label)? {
            core_syntax::SyntaxElement::Node(n) => Some(n),
            core_syntax::SyntaxElement::Token(_) => None,
        }
    }

    /// Does `n` have an own token (not trivia) with this text (`static`, `set`, `type`)?
    pub fn has_own(&self, n: &SyntaxNode, text: &str) -> bool {
        n.tokens()
            .any(|t| t.text() == text && !self.g.is_trivia(t.kind()))
    }

    /// The span of `n` without the trivia around it.
    pub fn range(&self, n: &SyntaxNode) -> TextRange {
        significant_range(n, self.g)
    }

    /// Is the import `name` the JSX pragma that the elements of the file use (TypeScript files only)?
    pub fn jsx_pragma(&self, name: &str) -> bool {
        self.ts && LANG.markup_import(name, &self.root, self.g)
    }

    /// The symbol a binding node declares.
    pub fn symbol_of(&self, binding: &SyntaxNode) -> Option<SymbolId> {
        let map = self.by_binding.get_or_init(|| {
            let mut map = HashMap::new();
            for id in self.s.symbol_ids() {
                for d in &self.s.symbol(id).decls {
                    map.insert(u32::from(d.name.start()), id);
                }
            }
            map
        });
        map.get(&u32::from(self.range(binding).start())).copied()
    }
}

thread_local! {
    static CACHE: FileCache<Cx> = const { FileCache::new() };
}

/// The model of the file, built once and shared by the rules that run on it; `None` when the
/// parse failed (the parser's diagnostic stands, a tree of one unknown token binds nothing).
pub fn model(file: &FileContext<'_>) -> Option<Rc<Cx>> {
    let cx = CACHE.with(|cache| {
        cache.get(file, || {
            let name = file.path.to_string_lossy();
            Cx {
                s: core_rules::scope_rules::model(&LANG, file),
                root: file.root.clone(),
                g: grammars::JS.get(),
                ts: source_type(&name).is_typescript(),
                by_binding: OnceCell::new(),
            }
        })
    });
    let fatal = file
        .root
        .first_token()
        .is_some_and(|t| t.kind() == k::UNKNOWN);
    (!fatal).then_some(cx)
}
