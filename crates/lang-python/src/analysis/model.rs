//! Scopes and bindings of one file: what each name refers to, and whether anything read it.
//! The shape follows pyflakes and ruff: the latest binding of a name is the one a read finds, a
//! rebinding in the same scope inherits what read the old one, and function bodies are read after
//! the scope around them is complete (the checker defers them).

use super::Hit;
use super::names::is_builtin;
use ruff_text_size::TextRange;
use rustc_hash::FxHashMap;

pub type ScopeId = usize;
pub type BindId = usize;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Import,
    SubmoduleImport,
    FromImport,
    FutureImport,
    FunctionDef,
    ClassDef,
    Assignment,
    NamedExpr,
    /// `x: int` with no value.
    Annotation,
    Argument,
    LoopVar,
    WithItem,
    /// `except E as name`: unbound when the handler ends.
    Except,
    Comprehension,
    TypeParam,
    /// A name a `global` statement adds to the module.
    Global,
}

impl Kind {
    pub fn is_import(self) -> bool {
        matches!(
            self,
            Kind::Import | Kind::SubmoduleImport | Kind::FromImport | Kind::FutureImport
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ScopeKind {
    Module,
    Class,
    Function {
        is_async: bool,
    },
    Lambda,
    Generator {
        is_gen: bool,
        is_async: bool,
    },
    /// The names of a PEP 695 type parameter list.
    Type,
}

impl ScopeKind {
    pub fn is_function(self) -> bool {
        matches!(self, ScopeKind::Function { .. } | ScopeKind::Lambda)
    }
}

pub struct Binding<'a> {
    pub kind: Kind,
    pub name: &'a str,
    /// What a finding points at: the name, or the alias of an import.
    pub range: TextRange,
    /// The `if`/`try`/`with`/`match` branch the binding was made in: bindings of different
    /// branches do not redefine each other.
    pub branch: u32,
    pub used: bool,
    /// The scope and range of the first read of the latest run of reads from one scope (F823).
    pub last_use: Option<(ScopeId, TextRange)>,
    /// Part of a tuple or list target whose value is not a literal sequence: not an unused
    /// variable.
    pub unpacked: bool,
    /// `import a as a`, `from m import a as a`: a re-export.
    pub explicit_export: bool,
    /// What an import binds, as written (`a.b` for `import a.b`; message and redefinition).
    pub label: Box<str>,
    /// What the bound name refers to (`a` for `import a.b`), for resolving `a.b.c`.
    pub target: Box<str>,
    /// A function decorated with `typing.overload`.
    pub overload: bool,
}

pub struct Scope<'a> {
    pub kind: ScopeKind,
    pub parent: Option<ScopeId>,
    /// The latest binding of each name.
    pub bindings: FxHashMap<&'a str, BindId>,
    /// Names declared `global` (true) or `nonlocal` here, with the declaration.
    pub declared: FxHashMap<&'a str, (bool, TextRange)>,
    /// The star imports of the scope: the module, the statement, whether a name came from it.
    pub stars: Vec<(String, TextRange, bool)>,
    pub uses_locals: bool,
    /// The names of `__all__` with the range of their string.
    pub exports: Vec<(String, TextRange)>,
    /// The branch the scope was opened in.
    pub branch: u32,
}

/// An import of an enclosing scope that a binding of a function scope covers (F811, F402).
struct Shadow {
    import: BindId,
    by: BindId,
}

/// What `Model::add` is told about a binding.
pub struct New {
    pub kind: Kind,
    pub range: TextRange,
    pub unpacked: bool,
    /// An import's label and target.
    pub names: Option<(Box<str>, Box<str>)>,
    pub explicit_export: bool,
    pub overload: bool,
}

impl New {
    pub fn of(kind: Kind, range: TextRange) -> Self {
        New {
            kind,
            range,
            unpacked: false,
            names: None,
            explicit_export: false,
            overload: false,
        }
    }
}

pub struct Model<'a> {
    pub scopes: Vec<Scope<'a>>,
    pub bindings: Vec<Binding<'a>>,
    /// The scopes around the code being read, outermost first.
    pub stack: Vec<ScopeId>,
    pub branch: u32,
    next_branch: u32,
    shadows: Vec<Shadow>,
    pub hits: Vec<Hit>,
}

pub enum Found {
    Bound(BindId),
    Builtin,
    Missing,
    /// Only `x: int` with no value was found: the name is unbound, whatever stars are around.
    Unbound(BindId),
}

/// `_`, `__`, `_ignored`: the names ruff's `dummy-variable-rgx` accepts.
pub fn is_dummy(name: &str) -> bool {
    name.starts_with('_')
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        && (name.bytes().all(|b| b == b'_') || name.ends_with(|c: char| c.is_ascii_alphanumeric()))
}

/// Does a new binding of `kind` and `label` redefine the definition `old`?
fn redefines(kind: Kind, label: &str, old: &Binding<'_>) -> bool {
    match kind {
        Kind::Annotation | Kind::FutureImport | Kind::Global => false,
        Kind::Import | Kind::FromImport | Kind::SubmoduleImport => match (kind, old.kind) {
            (Kind::Import | Kind::FromImport, Kind::SubmoduleImport)
            | (Kind::SubmoduleImport, Kind::Import | Kind::SubmoduleImport) => *old.label == *label,
            _ => true,
        },
        _ => true,
    }
}

impl<'a> Model<'a> {
    pub fn new() -> Self {
        let mut m = Model {
            scopes: Vec::new(),
            bindings: Vec::new(),
            stack: Vec::new(),
            branch: 0,
            next_branch: 1,
            shadows: Vec::new(),
            hits: Vec::new(),
        };
        m.push_scope(ScopeKind::Module);
        m
    }

    pub fn hit(&mut self, check: &'static str, range: TextRange, name: impl Into<String>) {
        self.hits.push(Hit {
            check,
            start: range.start().to_u32(),
            end: range.end().to_u32(),
            name: name.into(),
        });
    }

    pub fn push_scope(&mut self, kind: ScopeKind) -> ScopeId {
        let id = self.scopes.len();
        self.scopes.push(Scope {
            kind,
            parent: self.stack.last().copied(),
            bindings: FxHashMap::default(),
            declared: FxHashMap::default(),
            stars: Vec::new(),
            uses_locals: false,
            exports: Vec::new(),
            branch: self.branch,
        });
        self.stack.push(id);
        id
    }

    pub fn pop_scope(&mut self) {
        self.stack.pop();
    }

    pub fn current(&self) -> ScopeId {
        *self.stack.last().expect("a scope is always open")
    }

    pub fn new_branch(&mut self) -> u32 {
        self.branch = self.next_branch;
        self.next_branch += 1;
        self.branch
    }

    /// The binding a read of `name` finds, without marking it read.
    pub fn lookup(&self, name: &str) -> Found {
        let innermost = self.current();
        let mut unbound = None;
        for &sid in self.stack.iter().rev() {
            let scope = &self.scopes[sid];
            if scope.kind == ScopeKind::Class && sid != innermost {
                continue;
            }
            if let Some(&bid) = scope.bindings.get(name) {
                if self.bindings[bid].kind != Kind::Annotation {
                    return Found::Bound(bid);
                }
                if scope.kind != ScopeKind::Class {
                    unbound.get_or_insert(bid);
                }
            }
        }
        match unbound {
            _ if is_builtin(name) => Found::Builtin,
            Some(b) => Found::Unbound(b),
            None => Found::Missing,
        }
    }

    pub fn mark_used(&mut self, bid: BindId, range: TextRange) {
        let scope = self.current();
        let b = &mut self.bindings[bid];
        b.used = true;
        // The first read of a run in one scope: that is where F823 points.
        if b.last_use.is_none_or(|(s, _)| s != scope) {
            b.last_use = Some((scope, range));
        }
    }

    /// The scope a binding of `name` made here goes to: the module or the enclosing function for
    /// a name declared `global` or `nonlocal`.
    fn target_scope(&self, here: ScopeId, name: &str) -> ScopeId {
        let Some(&(is_global, _)) = self.scopes[here].declared.get(name) else {
            return here;
        };
        if is_global {
            return 0;
        }
        let mut at = self.scopes[here].parent;
        while let Some(s) = at {
            if self.scopes[s].kind.is_function() && self.scopes[s].bindings.contains_key(name) {
                return s;
            }
            at = self.scopes[s].parent;
        }
        here
    }

    /// Binds `name` in the current scope (or where a declaration sends it).
    pub fn add(&mut self, name: &'a str, new: New) -> Option<BindId> {
        // A walrus in a comprehension binds in the scope around it.
        let here = if new.kind == Kind::NamedExpr {
            (self.stack.iter().rev())
                .find(|&&s| !matches!(self.scopes[s].kind, ScopeKind::Generator { .. }))
                .copied()
                .unwrap_or(0)
        } else {
            self.current()
        };
        let scope = self.target_scope(here, name);
        let existing = self.scopes[scope].bindings.get(name).copied();
        // An annotation is no value: it never replaces one.
        if new.kind == Kind::Annotation && existing.is_some() {
            return None;
        }
        let (label, target) = new.names.unwrap_or_default();
        if let Some(old) = existing {
            // Through `global`, ruff leaves imports alone and reports the earlier definition,
            // not the assignment.
            if scope == here {
                self.redefinition(old, &new.kind, new.range, name, &label);
            } else if !self.bindings[old].kind.is_import() {
                self.redefinition(old, &new.kind, self.bindings[old].range, name, &label);
            }
        } else if scope == here {
            self.shadow_check(name, new.kind);
            if !matches!(
                new.kind,
                Kind::Annotation
                    | Kind::Argument
                    | Kind::Except
                    | Kind::FutureImport
                    | Kind::Global
            ) {
                self.undefined_local(name, scope);
            }
        }
        if existing.is_some() && scope != here {
            // Assigned through `global`: the module keeps the binding it has.
            return None;
        }
        let id = self.bindings.len();
        self.bindings.push(Binding {
            kind: new.kind,
            name,
            range: new.range,
            branch: self.branch,
            used: existing.is_some_and(|o| self.bindings[o].used)
                || scope != here
                || self.scopes[here].declared.contains_key(name),
            last_use: existing.and_then(|o| self.bindings[o].last_use),
            unpacked: new.unpacked,
            explicit_export: new.explicit_export,
            label,
            target,
            overload: new.overload,
        });
        self.scopes[scope].bindings.insert(name, id);
        Some(id)
    }

    /// F811 and F402 against the binding the name had in the scope.
    fn redefinition(
        &mut self,
        old: BindId,
        kind: &Kind,
        range: TextRange,
        name: &str,
        label: &str,
    ) {
        let o = &self.bindings[old];
        if o.branch != self.branch {
            return;
        }
        let was_import = o.kind.is_import();
        if *kind == Kind::LoopVar && was_import {
            self.hit("import-shadowed-by-loop-var", range, name);
            return;
        }
        // An import, `def` or `class` also redefines an unread assignment.
        let definition = was_import
            || matches!(o.kind, Kind::FunctionDef | Kind::ClassDef)
            || (matches!(o.kind, Kind::Assignment | Kind::NamedExpr)
                && (kind.is_import() || matches!(kind, Kind::FunctionDef | Kind::ClassDef)));
        if o.used || !definition || o.overload || o.kind == Kind::FutureImport {
            return;
        }
        if redefines(*kind, label, o) && (!is_dummy(name) || was_import) {
            self.hit("redefined-while-unused", range, name);
        }
    }

    /// A binding made in a function scope over an import of an enclosing scope: reported at the
    /// end, when it is known whether the import was read. Neither binding may sit in a branch
    /// (`if`, `try`, `with`, `match`) of its scope.
    fn shadow_check(&mut self, name: &str, kind: Kind) {
        let here = self.current();
        if matches!(self.scopes[here].kind, ScopeKind::Class | ScopeKind::Module)
            || self.branch != self.scopes[here].branch
            || matches!(kind, Kind::Annotation | Kind::FutureImport | Kind::Global)
        {
            return;
        }
        let mut at = self.scopes[here].parent;
        while let Some(s) = at {
            if let Some(&bid) = self.scopes[s].bindings.get(name) {
                if self.bindings[bid].kind.is_import()
                    && self.bindings[bid].branch == self.scopes[s].branch
                {
                    self.shadows.push(Shadow {
                        import: bid,
                        by: self.bindings.len(),
                    });
                }
                return;
            }
            at = self.scopes[s].parent;
        }
    }

    /// The shadows still unread at the end.
    pub fn report_shadows(&mut self) {
        let shadows = std::mem::take(&mut self.shadows);
        for s in shadows {
            let (old, by) = (&self.bindings[s.import], &self.bindings[s.by]);
            if old.kind == Kind::FutureImport {
                continue;
            }
            let (name, range) = (by.name, by.range);
            if by.kind == Kind::LoopVar {
                self.hit("import-shadowed-by-loop-var", range, name);
            } else if !old.used && redefines(by.kind, &by.label, old) {
                self.hit("redefined-while-unused", range, name);
            }
        }
    }

    /// F823: a name assigned in a function that an earlier read of the same function found in an
    /// enclosing scope.
    fn undefined_local(&mut self, name: &str, scope: ScopeId) {
        if !matches!(self.scopes[scope].kind, ScopeKind::Function { .. })
            || self.scopes[scope].declared.contains_key(name)
        {
            return;
        }
        let mut at = self.scopes[scope].parent;
        while let Some(s) = at {
            let sc = &self.scopes[s];
            if matches!(sc.kind, ScopeKind::Function { .. } | ScopeKind::Module)
                && let Some(&bid) = sc.bindings.get(name)
            {
                if let Some((user, range)) = self.bindings[bid].last_use
                    && user == scope
                {
                    self.hit("undefined-local", range, name);
                }
                return;
            }
            at = sc.parent;
        }
    }
}
