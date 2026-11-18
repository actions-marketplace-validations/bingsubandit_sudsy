//! The scope model of a file, for the rules that need to know what a name binds to: the shared
//! binder of `core_semantics::scopes` run over the tree with Python's table (which kinds open
//! scopes, declare names, refer to names or assign). Built on the first scope rule that runs on a
//! file and shared with the others (docs/ai/formatting/python.md#scope-layer).
//!
//! Resolution is by name up the scope chain, whatever the order of the code: `x` read at the top of
//! a module finds an `x` assigned below it. What depends on the order (a redefinition before a
//! read, a read before an assignment in a function) the rules read off the positions of the
//! declarations and references of a symbol.
//!
//! concept: scope-tables

use crate::generated::kind as k;
use core_rules::FileContext;
use core_rules::ScopeLang;
use core_rules::checks::FileCache;
use core_semantics::scopes::Model;
use core_syntax::{Grammar, SyntaxNode, TextRange, TextSize, grammars};
use rustc_hash::FxHashMap as HashMap;
use std::cell::{OnceCell, RefCell};
use std::rc::Rc;

/// The last name of a dotted expression: `Optional` for `typing.Optional`.
pub fn last_name(n: &SyntaxNode) -> Option<String> {
    last_name_token(n).map(|t| t.text().to_owned())
}

fn last_name_token(n: &SyntaxNode) -> Option<core_syntax::SyntaxToken> {
    let start = n.text_range().start();
    let mut t = n.last_token();
    while let Some(tok) = t {
        if tok.kind() == k::NAME {
            return Some(tok);
        }
        if tok.text_range().start() <= start {
            return None;
        }
        t = tok.prev_token();
    }
    None
}

/// The scope rules' view of Python: the table above and the facts the shared rules ask for
/// (the order of the code decides what a name means: `core_rules::scope_rules`).
pub static LANG: ScopeLang =
    ScopeLang::new(include_str!("../scope.rows")).globals(include_str!("../globals/python.txt"));

/// The start and kind of a node.
type NodeKey = (u32, u32);

/// What a scope rule reads: the model and the facts of the file.
pub struct Cx {
    pub s: Rc<Model>,
    pub root: SyntaxNode,
    pub g: &'static Grammar,
    /// The minor version of the target (`3.14`: 14).
    pub minor: u32,
    by_ref: OnceCell<HashMap<u32, usize>>,
    /// What [`Cx::qualified`] found, by the start and kind of the node asked about.
    qualified: RefCell<HashMap<NodeKey, Option<Rc<str>>>>,
}

thread_local! {
    static CACHE: FileCache<Option<Rc<Cx>>> = const { FileCache::new() };
    /// The file parsed last on this thread had syntax errors: its text's length and hash, and
    /// the target version's minor number.
    static NOTE: RefCell<Option<Note>> = const { RefCell::new(None) };
}

struct Note {
    len: usize,
    hash: u64,
    errors: bool,
    minor: u32,
}

fn hash(text: &str) -> u64 {
    use std::hash::Hasher;
    let mut h = rustc_hash::FxHasher::default();
    h.write(text.as_bytes());
    h.finish()
}

/// What the parse of `text` found, for the rules that follow on this thread.
pub fn note_parse(text: &str, errors: bool, minor: u32) {
    NOTE.with(|n| {
        *n.borrow_mut() = Some(Note {
            len: text.len(),
            hash: hash(text),
            errors,
            minor,
        });
    });
}

/// `(errors, minor)` of the last parse when it was of `text`.
fn noted(text: &str) -> Option<(bool, u32)> {
    NOTE.with(|n| {
        let n = n.borrow();
        let n = n.as_ref()?;
        (n.len == text.len() && n.hash == hash(text)).then_some((n.errors, n.minor))
    })
}

/// The model of the file, built once and shared by the rules that run on it; `None` for a file
/// with syntax errors (the parser's diagnostic stands).
pub fn model(file: &FileContext<'_>) -> Option<Rc<Cx>> {
    // Every rule asks: the answer (also "none") is cached, so the text is hashed once per file.
    CACHE.with(|cache| {
        cache
            .get(file, || {
                let (errors, minor) = noted(file.text).unwrap_or((false, 14));
                let fatal = file
                    .root
                    .first_token()
                    .is_some_and(|t| t.kind() == k::UNKNOWN);
                if errors || fatal {
                    return None;
                }
                Some(Rc::new(Cx {
                    s: core_rules::scope_rules::model(&LANG, file),
                    root: file.root.clone(),
                    g: grammars::PYTHON.get(),
                    minor,
                    by_ref: OnceCell::new(),
                    qualified: RefCell::default(),
                }))
            })
            .as_ref()
            .clone()
    })
}

impl Cx {
    /// The child node in the body item `label` (a field of the grammar).
    pub fn slot(&self, n: &SyntaxNode, label: &str) -> Option<SyntaxNode> {
        self.g.slot(n, label)?.into_node()
    }

    /// The span of `n` without the trivia around it.
    pub fn range(&self, n: &SyntaxNode) -> TextRange {
        core_rules::tree::significant_range(n, self.g)
    }

    /// The reference whose name starts at `start`.
    pub fn ref_at(&self, start: TextSize) -> Option<&core_semantics::scopes::Reference> {
        let map = self.by_ref.get_or_init(|| {
            (self.s.refs.iter().enumerate())
                .map(|(i, r)| (u32::from(r.range.start()), i))
                .collect()
        });
        map.get(&u32::from(start)).map(|i| &self.s.refs[*i])
    }

    /// The name a reference to a dotted name stands for through imports: `typing.Literal` for
    /// `Literal` imported from typing, `print` for the builtin; `None` for anything else.
    pub fn qualified(&self, node: &SyntaxNode) -> Option<String> {
        let key = (u32::from(node.text_range().start()), node.kind().0);
        if let Some(found) = self.qualified.borrow().get(&key) {
            return found.as_deref().map(str::to_owned);
        }
        let found = self.qualify(node);
        self.qualified
            .borrow_mut()
            .insert(key, found.as_deref().map(Rc::from));
        found
    }

    fn qualify(&self, node: &SyntaxNode) -> Option<String> {
        match node.kind() {
            k::EXPR_NAME => {
                let r = self.ref_at(self.range(node).start())?;
                match r.symbol {
                    Some(sym) => {
                        let d = self.s.symbol(sym).decls.last()?;
                        LANG.import_target(self.g, &self.root, d)
                    }
                    None => LANG
                        .is_global(&r.name, std::path::Path::new(""))
                        .then(|| r.name.to_string()),
                }
            }
            k::EXPR_ATTRIBUTE => {
                let object = self.slot(node, "object")?;
                let member = self.slot(node, "member")?;
                Some(format!(
                    "{}.{}",
                    self.qualified(&object)?,
                    last_name(&member)?
                ))
            }
            k::PARENTHESIZED => self.qualified(&node.first_child()?),
            _ => None,
        }
    }
}
