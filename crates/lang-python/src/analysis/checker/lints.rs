//! Checks of ruff's other families (flake8-bugbear, flake8-simplify, pyupgrade, flake8-pie,
//! flake8-comprehensions, flake8-print) that read the syntax and the names a call or a class
//! refers to. Each is a hook of the walk: `expr` and `stmt` run before a node is read, `handler`
//! for an `except` clause, `class` and `defaults` for definitions.

use super::Checker;
use ruff_python_ast::{self as ast, CmpOp, Expr, Operator, PythonVersion, Stmt, UnaryOp};
use ruff_text_size::Ranged;
use rustc_hash::FxHashMap;

const KEYWORDS: &[&str] = &[
    "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class", "continue",
    "def", "del", "elif", "else", "except", "finally", "for", "from", "global", "if", "import",
    "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while",
    "with", "yield",
];

/// Names that stand for `OSError`.
const OS_ERROR_ALIASES: &[&str] = &[
    "EnvironmentError",
    "IOError",
    "WindowsError",
    "mmap.error",
    "os.error",
    "select.error",
    "socket.error",
];

/// Calls that build a mutable value.
const MUTABLE_CALLS: &[&str] = &[
    "list",
    "dict",
    "set",
    "collections.Counter",
    "collections.OrderedDict",
    "collections.defaultdict",
    "collections.deque",
];

/// Types of an annotation that say a default is not mutated (`Sequence`, `tuple`, ...).
const IMMUTABLE_TYPES: &[&str] = &[
    "bool",
    "bytes",
    "complex",
    "float",
    "frozenset",
    "int",
    "str",
    "tuple",
    "typing.AbstractSet",
    "typing.ByteString",
    "typing.Collection",
    "typing.Container",
    "typing.Final",
    "typing.FrozenSet",
    "typing.Hashable",
    "typing.Iterable",
    "typing.Iterator",
    "typing.Literal",
    "typing.Mapping",
    "typing.Reversible",
    "typing.Sequence",
    "typing.Sized",
    "typing.Tuple",
    "collections.abc.Set",
    "collections.abc.Collection",
    "collections.abc.Container",
    "collections.abc.Hashable",
    "collections.abc.Iterable",
    "collections.abc.Iterator",
    "collections.abc.Mapping",
    "collections.abc.Reversible",
    "collections.abc.Sequence",
    "collections.abc.Sized",
];

fn is_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    chars
        .next()
        .is_some_and(|c| c == '_' || unicode_ident::is_xid_start(c))
        && chars.all(unicode_ident::is_xid_continue)
        && !KEYWORDS.contains(&s)
}

/// The attribute of a `getattr`/`setattr` call: a plain string that is a name (not a mangled
/// private one).
fn attribute_name(e: &Expr) -> bool {
    matches!(e, Expr::StringLiteral(s) if {
        let v = s.value.to_str();
        is_identifier(v) && v != "__debug__" && !(v.starts_with("__") && !v.ends_with("__"))
    })
}

fn is_none(e: &Expr) -> bool {
    matches!(e, Expr::NoneLiteral(_))
}

impl<'a> Checker<'a> {
    fn text(&self, e: &Expr) -> &'a str {
        &self.src[e.range()]
    }

    /// Hooks that read an expression before it is walked.
    pub(super) fn lint_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Call(c) => self.lint_call(c),
            Expr::Compare(c) => self.lint_compare(c),
            Expr::UnaryOp(u) => self.lint_unary(u),
            Expr::If(i) => self.lint_if(i),
            Expr::BoolOp(b) => self.lint_bool_op(b),
            Expr::StringLiteral(s) => {
                for part in &s.value {
                    if part.flags.prefix().is_unicode() {
                        self.m.hit("unicode-literal", part.range, "");
                    }
                }
            }
            Expr::FString(f) => {
                for part in &f.value {
                    if let ast::FStringPartRef::Literal(l) = part
                        && l.flags.prefix().is_unicode()
                    {
                        self.m.hit("unicode-literal", l.range, "");
                    }
                }
            }
            Expr::ListComp(c) => self.lint_generators(&c.generators),
            Expr::SetComp(c) => self.lint_generators(&c.generators),
            Expr::DictComp(c) => self.lint_generators(&c.generators),
            Expr::Generator(c) => self.lint_generators(&c.generators),
            _ => {}
        }
    }

    fn lint_call(&mut self, c: &'a ast::ExprCall) {
        let args = &c.arguments.args;
        let keywords = &c.arguments.keywords;
        let name = match &*c.func {
            Expr::Name(f) => f.id.as_str(),
            Expr::Attribute(_) => "",
            _ => return,
        };
        if !matches!(
            name,
            "getattr" | "print" | "zip" | "dict" | "list" | "tuple"
        ) {
            if self.qualified(&c.func).as_deref() == Some("pprint.pprint")
                && self.writes_to_console(&c.arguments, "stream")
            {
                self.m.hit("pprint", c.range(), "");
            }
            return;
        }
        if !self.is_builtin_name(name) {
            return;
        }
        match name {
            "getattr" if args.len() == 2 && keywords.is_empty() && attribute_name(&args[1]) => {
                self.m.hit("get-attr-with-constant", c.range(), "");
            }
            "print" => {
                if self.writes_to_console(&c.arguments, "file") {
                    self.m.hit("print", c.range(), "");
                }
            }
            "zip" if self.target >= PythonVersion::PY310 => {
                let strict = keywords
                    .iter()
                    .any(|k| k.arg.as_ref().is_some_and(|a| a.as_str() == "strict"));
                let starred = args.iter().any(Expr::is_starred_expr);
                if !strict
                    && (args.len() > 1 || starred)
                    && !args.iter().any(|a| self.is_infinite(a))
                {
                    self.m.hit("zip-without-explicit-strict", c.range(), "");
                }
            }
            "dict" if args.is_empty() && keywords.iter().all(|k| k.arg.is_some()) => {
                self.m.hit("unnecessary-collection-call", c.range(), name);
            }
            "list" | "tuple" if args.is_empty() && keywords.is_empty() => {
                self.m.hit("unnecessary-collection-call", c.range(), name);
            }
            _ => {}
        }
    }

    /// Does the call print to the console: no `file` (or `stream`) argument, `None`, or
    /// `sys.stdout` or `sys.stderr`?
    fn writes_to_console(&self, arguments: &ast::Arguments, keyword: &str) -> bool {
        arguments
            .keywords
            .iter()
            .find(|k| k.arg.as_ref().is_some_and(|a| a.as_str() == keyword))
            .is_none_or(|k| {
                is_none(&k.value)
                    || self
                        .qualified(&k.value)
                        .is_some_and(|q| matches!(q.as_str(), "sys.stdout" | "sys.stderr"))
            })
    }

    /// `itertools.count()`, `cycle(x)` or `repeat(x)` (without a count): never ends.
    fn is_infinite(&self, e: &Expr) -> bool {
        let Expr::Call(c) = e else {
            return false;
        };
        let n = c.arguments.args.len() + c.arguments.keywords.len();
        match self.qualified(&c.func).as_deref() {
            Some("itertools.count" | "itertools.cycle") => true,
            Some("itertools.repeat") => n == 1,
            _ => false,
        }
    }

    /// SIM118: `key in dict.keys()`.
    fn lint_compare(&mut self, c: &ast::ExprCompare) {
        if let ([CmpOp::In | CmpOp::NotIn], Some(right)) = (&*c.ops, c.operands.get(1))
            && is_keys_call(right)
        {
            self.m.hit("in-dict-keys", c.range(), "");
        }
    }

    fn lint_generators(&mut self, generators: &[ast::Comprehension]) {
        for g in generators {
            if is_keys_call(&g.iter) {
                self.m.hit("in-dict-keys", g.range(), "");
            }
        }
    }

    /// SIM201, SIM202 and SIM208: a negated comparison or a double negation.
    fn lint_unary(&mut self, u: &ast::ExprUnaryOp) {
        if u.op != UnaryOp::Not {
            return;
        }
        let skip = self.exception_check || matches!(self.fn_name, "__eq__" | "__ne__");
        match &*u.operand {
            Expr::Compare(c) if c.ops.len() == 1 && !skip => match c.ops[0] {
                CmpOp::Eq => self.m.hit("negate-equal-op", u.range(), ""),
                CmpOp::NotEq => self.m.hit("negate-not-equal-op", u.range(), ""),
                _ => {}
            },
            Expr::UnaryOp(inner) if inner.op == UnaryOp::Not => {
                self.m.hit("double-negation", u.range(), "");
            }
            _ => {}
        }
    }

    /// SIM210, SIM211 and SIM212: a conditional expression that is a boolean or a swapped one.
    fn lint_if(&mut self, i: &ast::ExprIf) {
        let boolean = |e: &Expr| match e {
            Expr::BooleanLiteral(b) => Some(b.value),
            _ => None,
        };
        match (boolean(&i.body), boolean(&i.orelse)) {
            (Some(true), Some(false)) => self.m.hit("if-expr-with-true-false", i.range(), ""),
            (Some(false), Some(true)) => self.m.hit("if-expr-with-false-true", i.range(), ""),
            _ => {
                if let Expr::UnaryOp(u) = &*i.test
                    && u.op == UnaryOp::Not
                    && let (Expr::Name(a), Expr::Name(b)) = (&*u.operand, &*i.orelse)
                    && a.id == b.id
                {
                    self.m.hit("if-expr-with-twisted-arms", i.range(), "");
                }
            }
        }
    }

    /// PIE810: `a.startswith(x) or a.startswith(y)`.
    fn lint_bool_op(&mut self, b: &ast::ExprBoolOp) {
        if b.op != ast::BoolOp::Or {
            return;
        }
        let mut seen: FxHashMap<(&str, &str), u32> = FxHashMap::default();
        for v in &b.values {
            if let Expr::Call(c) = v
                && let Expr::Attribute(a) = &*c.func
                && matches!(a.attr.as_str(), "startswith" | "endswith")
                && c.arguments.args.len() == 1
                && c.arguments.keywords.is_empty()
            {
                *seen
                    .entry((a.attr.as_str(), self.text(&a.value)))
                    .or_default() += 1;
            }
        }
        let mut names: Vec<&str> = seen
            .iter()
            .filter(|(_, n)| **n > 1)
            .map(|(k, _)| k.0)
            .collect();
        names.sort_unstable();
        for name in names {
            self.m.hit("multiple-starts-ends-with", b.range(), name);
        }
    }

    /// Hooks that read a statement before it is walked.
    pub(super) fn lint_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::Assert(a) if matches!(&*a.test, Expr::BooleanLiteral(b) if !b.value) => {
                self.m.hit("assert-false", a.range(), "");
            }
            Stmt::Expr(e) => {
                if let Expr::Call(c) = &*e.value
                    && matches!(&*c.func, Expr::Name(n) if n.id.as_str() == "setattr")
                    && c.arguments.args.len() == 3
                    && c.arguments.keywords.is_empty()
                    && attribute_name(&c.arguments.args[1])
                    && self.is_builtin_name("setattr")
                {
                    self.m.hit("set-attr-with-constant", c.range(), "");
                }
            }
            Stmt::For(f) if is_keys_call(&f.iter) => {
                self.m.hit("in-dict-keys", f.iter.range(), "");
            }
            Stmt::Raise(r) => {
                if let Some(exc) = r.exc.as_deref() {
                    let target = match exc {
                        Expr::Call(c) => &*c.func,
                        e => e,
                    };
                    self.os_error_alias(target);
                }
            }
            _ => {}
        }
    }

    /// PIE790: a `pass` or `...` in a body that has other statements.
    pub(super) fn lint_body(&mut self, body: &[Stmt]) {
        if body.len() < 2 {
            return;
        }
        for s in body {
            match s {
                Stmt::Pass(p) => self.m.hit("unnecessary-placeholder", p.range(), "pass"),
                Stmt::Expr(e)
                    if !self.ellipsis_ok && matches!(&*e.value, Expr::EllipsisLiteral(_)) =>
                {
                    self.m.hit("unnecessary-placeholder", e.range(), "...");
                }
                _ => {}
            }
        }
    }

    /// Is the class a `Protocol`?
    pub(super) fn is_protocol(&self, c: &ast::StmtClassDef) -> bool {
        c.arguments.as_ref().is_some_and(|a| {
            a.args.iter().any(|b| {
                let b = match b {
                    Expr::Subscript(s) => &*s.value,
                    b => b,
                };
                self.qualified(b).is_some_and(|q| {
                    matches!(q.as_str(), "typing.Protocol" | "typing_extensions.Protocol")
                })
            })
        })
    }

    /// Is the function decorated with `abc.abstractmethod`?
    pub(super) fn is_abstract(&self, f: &ast::StmtFunctionDef) -> bool {
        f.decorator_list
            .iter()
            .any(|d| self.qualified(&d.expression).as_deref() == Some("abc.abstractmethod"))
    }

    /// UP004: `class A(object)`.
    pub(super) fn lint_class(&mut self, c: &ast::StmtClassDef) {
        if let Some(args) = &c.arguments {
            for a in &args.args {
                if matches!(a, Expr::Name(n) if n.id.as_str() == "object")
                    && self.is_builtin_name("object")
                {
                    self.m
                        .hit("useless-object-inheritance", a.range(), c.name.as_str());
                }
            }
        }
    }

    /// UP024: a name that stands for `OSError`.
    fn os_error_alias(&mut self, e: &Expr) {
        if let Some(q) = self.qualified(e)
            && OS_ERROR_ALIASES.contains(&q.as_str())
        {
            self.m.hit("os-error-alias", e.range(), q);
        }
    }

    /// B006: a default that is mutable and shared by every call.
    pub(super) fn lint_defaults(&mut self, parameters: &'a ast::Parameters) {
        for p in parameters.iter_non_variadic_params() {
            let Some(d) = p.default.as_deref() else {
                continue;
            };
            let mutable = match d {
                Expr::List(_)
                | Expr::Dict(_)
                | Expr::Set(_)
                | Expr::ListComp(_)
                | Expr::SetComp(_)
                | Expr::DictComp(_) => true,
                Expr::Call(c) => self
                    .qualified(&c.func)
                    .is_some_and(|q| MUTABLE_CALLS.contains(&q.as_str())),
                _ => false,
            };
            let immutable = p
                .parameter
                .annotation
                .as_deref()
                .is_some_and(|a| self.immutable_annotation(a));
            if mutable && !immutable {
                self.m.hit("mutable-argument-default", d.range(), "");
            }
        }
    }

    fn immutable_annotation(&self, a: &Expr) -> bool {
        match a {
            Expr::Subscript(s) => match self.qualified(&s.value).as_deref() {
                Some("typing.Optional") => self.immutable_annotation(&s.slice),
                Some("typing.Union") => match &*s.slice {
                    Expr::Tuple(t) => t.iter().all(|e| self.immutable_annotation(e)),
                    e => self.immutable_annotation(e),
                },
                _ => self.immutable_annotation(&s.value),
            },
            Expr::BinOp(b) if b.op == Operator::BitOr => {
                self.immutable_annotation(&b.left) && self.immutable_annotation(&b.right)
            }
            Expr::NoneLiteral(_) => true,
            Expr::StringLiteral(_) => false,
            e => self
                .qualified(e)
                .is_some_and(|q| IMMUTABLE_TYPES.contains(&q.as_str())),
        }
    }

    /// B013, B904 and UP024 for an `except` clause.
    pub(super) fn lint_handler(&mut self, h: &'a ast::ExceptHandlerExceptHandler, star: bool) {
        if let Some(t) = h.type_.as_deref() {
            match t {
                Expr::Tuple(tuple) => {
                    if tuple.elts.len() == 1 && !tuple.elts[0].is_starred_expr() {
                        self.m
                            .hit("redundant-tuple-in-exception-handler", t.range(), "");
                    }
                    for e in &tuple.elts {
                        self.os_error_alias(e);
                    }
                }
                e => self.os_error_alias(e),
            }
        }
        let bound = h.name.as_ref().map(|n| n.as_str());
        let kind = if star { "except*" } else { "except" };
        let mut raises = Vec::new();
        collect_raises(&h.body, &mut raises);
        for r in raises {
            let Some(exc) = r.exc.as_deref() else {
                continue;
            };
            if r.cause.is_some() || matches!(exc, Expr::Name(n) if Some(n.id.as_str()) == bound) {
                continue;
            }
            self.m
                .hit("raise-without-from-inside-except", r.range(), kind);
        }
    }
}

/// `x.keys()` with no arguments.
fn is_keys_call(e: &Expr) -> bool {
    matches!(e, Expr::Call(c)
        if c.arguments.is_empty()
            && matches!(&*c.func, Expr::Attribute(a) if a.attr.as_str() == "keys"
                && !matches!(&*a.value, Expr::Name(n) if matches!(n.id.as_str(), "self" | "cls"))))
}

/// The `raise` statements of a handler's body: nested blocks are read (not the `else` of a loop),
/// `try` statements, functions and classes are not.
fn collect_raises<'b>(body: &'b [Stmt], out: &mut Vec<&'b ast::StmtRaise>) {
    for s in body {
        match s {
            Stmt::Raise(r) => out.push(r),
            Stmt::If(i) => {
                collect_raises(&i.body, out);
                for c in &i.elif_else_clauses {
                    collect_raises(&c.body, out);
                }
            }
            Stmt::For(f) => collect_raises(&f.body, out),
            Stmt::While(w) => collect_raises(&w.body, out),
            Stmt::With(w) => collect_raises(&w.body, out),
            Stmt::Match(m) => {
                for c in &m.cases {
                    collect_raises(&c.body, out);
                }
            }
            _ => {}
        }
    }
}
