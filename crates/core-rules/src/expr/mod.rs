//! The expression language embedded in the scalars of a data file (`${{ github.ref }}` of a
//! workflow): syntax, the contexts a key may read (a table by key path), the functions and their
//! arities and types, the types of the contexts (written as text), and the operand types of
//! comparisons. The language gives the tables; the parser and the checks are here.
//!
//! concept: expression-language
//! what: Syntax, names, availability and types of the expressions inside the scalars of a data file.
//! tools: actionlint expression

mod parse;
mod ty;

use crate::rule::FileContext;
use crate::shape::{Finding, leaves};
use core_syntax::{TextRange, embedded::size};
use parse::{Kind, Lit, Node};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::OnceLock;
use ty::{Ty, alternatives};

/// A function of the language.
pub struct Func {
    pub name: &'static str,
    /// The type text of each parameter (`a|b` alternatives).
    pub params: &'static [&'static str],
    /// The fewest arguments.
    pub min: usize,
    /// The last parameter repeats.
    pub variadic: bool,
    pub ret: &'static str,
    /// Only available where the place lists it.
    pub special: bool,
    /// The first argument is a format string with `{N}` placeholders.
    pub format: bool,
}

impl Func {
    pub const NONE: Func = Func {
        name: "",
        params: &[],
        min: 0,
        variadic: false,
        ret: "any",
        special: false,
        format: false,
    };
}

impl Place {
    pub const NONE: Place = Place {
        path: "",
        contexts: &[],
        specials: &[],
        implicit: false,
    };
}

/// What the expressions at a key path may use. `path` is a prefix pattern (`*` is one key,
/// `a|b` alternatives); the first place that matches a scalar's path is its place.
pub struct Place {
    pub path: &'static str,
    pub contexts: &'static [&'static str],
    pub specials: &'static [&'static str],
    /// The whole value is an expression, with no `${{ }}` around it (`if:`).
    pub implicit: bool,
}

/// The rule ids that report each class of finding.
pub struct Rules {
    pub syntax: &'static str,
    pub context: &'static str,
    pub types: &'static str,
}

pub struct Spec {
    pub open: &'static str,
    pub close: &'static str,
    /// Each context: its name and type text.
    pub contexts: &'static [(&'static str, &'static str)],
    pub functions: &'static [Func],
    pub places: &'static [Place],
    pub rules: Rules,
    pub compiled: OnceLock<Compiled>,
}

type Signature = (String, Vec<Vec<Ty>>, Ty);

pub struct Compiled {
    contexts: Vec<(String, Ty)>,
    functions: Vec<Signature>,
}

impl Spec {
    fn compiled(&self) -> &Compiled {
        self.compiled.get_or_init(|| Compiled {
            contexts: (self.contexts.iter())
                .map(|(n, t)| (n.to_ascii_lowercase(), Ty::parse(t)))
                .collect(),
            functions: (self.functions.iter())
                .map(|f| {
                    let params = f.params.iter().map(|p| alternatives(p)).collect();
                    (f.name.to_ascii_lowercase(), params, Ty::parse(f.ret))
                })
                .collect(),
        })
    }

    fn place(&self, path: &[String]) -> Option<&Place> {
        self.places.iter().find(|place| {
            place.path.split('|').any(|alt| {
                let pat: Vec<&str> = alt.split('.').filter(|s| !s.is_empty()).collect();
                pat.len() <= path.len() && pat.iter().zip(path).all(|(a, b)| *a == "*" || a == b)
            })
        })
    }
}

#[derive(Clone, Copy)]
enum Class {
    Syntax,
    Context,
    Types,
}

struct Checker<'a> {
    spec: &'a Spec,
    place: Option<&'a Place>,
    errors: Vec<(Class, usize, String)>,
}

fn quoted_list(names: impl Iterator<Item = String>) -> String {
    names
        .map(|n| format!("\"{n}\""))
        .collect::<Vec<_>>()
        .join(", ")
}

impl Checker<'_> {
    fn err(&mut self, class: Class, at: usize, message: String) {
        self.errors.push((class, at, message));
    }

    fn ty(&mut self, n: &Node) -> Ty {
        let c = self.spec.compiled();
        match &n.kind {
            Kind::Lit(Lit::Null) => Ty::Null,
            Kind::Lit(Lit::Bool(_)) => Ty::Bool,
            Kind::Lit(Lit::Num) => Ty::Num,
            Kind::Lit(Lit::Str(_)) => Ty::Str,
            Kind::Var(name) => {
                let lower = name.to_ascii_lowercase();
                let Some((_, t)) = c.contexts.iter().find(|(k, _)| *k == lower) else {
                    let names = quoted_list(c.contexts.iter().map(|c| c.0.clone()));
                    let message =
                        format!("undefined variable \"{name}\". available variables are {names}");
                    self.err(Class::Syntax, n.start, message);
                    return Ty::Any;
                };
                if let Some(place) = self.place
                    && !place.contexts.contains(&lower.as_str())
                {
                    let available = if place.contexts.is_empty() {
                        "no context is available here".to_string()
                    } else {
                        format!(
                            "available contexts are {}",
                            quoted_list(place.contexts.iter().map(|s| s.to_string()))
                        )
                    };
                    self.err(
                        Class::Context,
                        n.start,
                        format!("context \"{name}\" is not allowed here. {available}"),
                    );
                }
                t.clone()
            }
            Kind::Prop(obj, name) => {
                let t = self.ty(obj);
                self.property(n, &t, name)
            }
            Kind::Star(obj) => {
                self.ty(obj);
                Ty::Any
            }
            Kind::Index(obj, index) => {
                let (t, i) = (self.ty(obj), self.ty(index));
                match (&t, &i, &index.kind) {
                    (Ty::Arr(e), Ty::Num | Ty::Any, _) => (**e).clone(),
                    (Ty::Arr(_), _, _) => {
                        self.err(
                            Class::Types,
                            n.start,
                            format!("index access of array must be type of number but got \"{i}\""),
                        );
                        Ty::Any
                    }
                    (Ty::Obj(_) | Ty::Map(_), _, Kind::Lit(Lit::Str(name))) => {
                        self.property(n, &t, name)
                    }
                    (Ty::Obj(_) | Ty::Map(_), Ty::Str | Ty::Any, _) => Ty::Any,
                    (Ty::Obj(_) | Ty::Map(_), _, _) => {
                        self.err(
                            Class::Types,
                            n.start,
                            format!(
                                "property access of object must be type of string but got \"{i}\""
                            ),
                        );
                        Ty::Any
                    }
                    _ => Ty::Any,
                }
            }
            Kind::Call(name, args) => self.call(n, name, args),
            Kind::Not(inner) => {
                self.ty(inner);
                Ty::Bool
            }
            Kind::And(l, r) | Kind::Or(l, r) => {
                self.ty(l);
                self.ty(r);
                Ty::Any
            }
            Kind::Cmp(op, l, r) => {
                let (a, b) = (self.ty(l), self.ty(r));
                let collection = |t: &Ty| matches!(t, Ty::Arr(_) | Ty::Obj(_) | Ty::Map(_));
                let ordering = !matches!(*op, "==" | "!=");
                let bad = |t: &Ty| collection(t) || (ordering && matches!(t, Ty::Bool | Ty::Null));
                if bad(&a) || bad(&b) {
                    self.err(Class::Types, n.start, format!("\"{a}\" value cannot be compared to \"{b}\" value with \"{op}\" operator"));
                }
                Ty::Bool
            }
        }
    }

    fn property(&mut self, n: &Node, t: &Ty, name: &str) -> Ty {
        match t {
            Ty::Obj(_) | Ty::Map(_) | Ty::Any => match t.prop(name) {
                Some(p) => p,
                None => {
                    self.err(
                        Class::Types,
                        n.start,
                        format!("property \"{name}\" is not defined in object type {t}"),
                    );
                    Ty::Any
                }
            },
            _ => {
                self.err(Class::Types, n.start, format!("receiver of object dereference \"{name}\" must be type of object but got \"{t}\""));
                Ty::Any
            }
        }
    }

    fn call(&mut self, n: &Node, name: &str, args: &[Node]) -> Ty {
        let c = self.spec.compiled();
        let lower = name.to_ascii_lowercase();
        let types: Vec<Ty> = args.iter().map(|a| self.ty(a)).collect();
        let Some(i) = c.functions.iter().position(|f| f.0 == lower) else {
            let names = quoted_list(self.spec.functions.iter().map(|f| f.name.to_string()));
            self.err(
                Class::Syntax,
                n.start,
                format!("undefined function \"{name}\". available functions are {names}"),
            );
            return Ty::Any;
        };
        let (def, (_, params, ret)) = (&self.spec.functions[i], &c.functions[i]);
        if def.special
            && let Some(place) = self.place
            && !place.specials.iter().any(|s| s.eq_ignore_ascii_case(name))
        {
            let message = format!(
                "calling function \"{name}\" is not allowed here. \"{name}\" is only available in some keys"
            );
            self.err(Class::Context, n.start, message);
        }
        let wrong = args.len() < def.min || (args.len() > params.len() && !def.variadic);
        if wrong {
            let message = format!(
                "number of arguments is wrong. function \"{name}\" takes {} parameters but {} arguments are given",
                params.len(),
                args.len()
            );
            self.err(Class::Types, n.start, message);
            return ret.clone();
        }
        for (k, (arg, t)) in args.iter().zip(&types).enumerate() {
            let Some(options) = params.get(k).or(params.last()) else {
                continue;
            };
            if !options.iter().any(|o| t.assignable(o)) {
                let message = format!(
                    "argument {} of function call is not assignable. \"{t}\" cannot be assigned",
                    k + 1
                );
                self.err(Class::Types, arg.start, message);
            }
        }
        if def.format
            && let Some(Node {
                kind: Kind::Lit(Lit::Str(f)),
                ..
            }) = args.first()
        {
            self.format_args(n, f, args.len() - 1);
        }
        ret.clone()
    }

    fn format_args(&mut self, n: &Node, format: &str, given: usize) {
        let mut max: Option<usize> = None;
        let mut rest = format.replace("{{", "").replace("}}", "");
        while let Some(open) = rest.find('{') {
            let Some(close) = rest[open..].find('}') else {
                break;
            };
            if let Ok(k) = rest[open + 1..open + close].trim().parse::<usize>() {
                max = Some(max.map_or(k, |m| m.max(k)));
            }
            rest = rest[open + close + 1..].to_string();
        }
        match max {
            Some(m) if m >= given => {
                let message = format!(
                    "format string \"{format}\" contains placeholder {{{m}}} but only {given} arguments are given to format"
                );
                self.err(Class::Types, n.start, message);
            }
            Some(m) if given > m + 1 => {
                let message = format!(
                    "format string \"{format}\" does not contain placeholder {{{}}}",
                    m + 1
                );
                self.err(Class::Types, n.start, message);
            }
            None if given > 0 => {
                let message =
                    format!("format string \"{format}\" does not contain placeholder {{0}}");
                self.err(Class::Types, n.start, message);
            }
            _ => {}
        }
    }
}

/// The end of the expression whose text starts at `from`: the offset of its close marker; strings
/// are skipped.
fn end_of(body: &str, from: usize, close: &str) -> Option<usize> {
    let bytes = body.as_bytes();
    let mut i = from;
    while i < bytes.len() {
        if body[i..].starts_with(close) {
            return Some(i);
        }
        if bytes[i] == b'\'' {
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\'' {
                    if bytes.get(i + 1) == Some(&b'\'') {
                        i += 1;
                    } else {
                        break;
                    }
                }
                i += 1;
            }
        }
        i += 1;
    }
    None
}

struct Run<'a> {
    spec: &'a Spec,
    out: Vec<Finding>,
}

impl Run<'_> {
    fn report(&mut self, class: Class, base: usize, at: usize, message: String) {
        self.report_len(class, base, at, 1, message);
    }

    fn report_len(&mut self, class: Class, base: usize, at: usize, len: usize, message: String) {
        let rule = match class {
            Class::Syntax => self.spec.rules.syntax,
            Class::Context => self.spec.rules.context,
            Class::Types => self.spec.rules.types,
        };
        let start = base + at;
        let range = TextRange::new(size(start), size(start + len));
        self.out.push(Finding {
            rule,
            range,
            message,
        });
    }

    /// Checks one expression whose text is `body`, found at `base` of the file.
    fn expression(&mut self, body: &str, base: usize, place: Option<&Place>, embedded: bool) {
        let node = match parse::parse(body) {
            Ok(n) => n,
            Err(e) => return self.report(Class::Syntax, base, e.at, e.message),
        };
        let mut checker = Checker {
            spec: self.spec,
            place,
            errors: Vec::new(),
        };
        let t = checker.ty(&node);
        if embedded && matches!(t, Ty::Arr(_) | Ty::Obj(_) | Ty::Map(_)) {
            checker.err(
                Class::Types,
                node.start,
                format!("object, array, and null values should not be evaluated in template with the expression markers but evaluating the value of type {t}"),
            );
        }
        for (class, at, message) in checker.errors {
            self.report(class, base, at, message);
        }
    }

    fn scalar(&mut self, text: &str, start: usize, place: Option<&Place>) {
        let (open, close) = (self.spec.open, self.spec.close);
        if place.is_some_and(|p| p.implicit) && !text.contains(open) {
            return self.expression(text, start, place, false);
        }
        let mut from = 0;
        while let Some(i) = text[from..].find(open).map(|i| i + from) {
            let Some(end) = end_of(text, i + open.len(), close) else {
                let message = format!("the expression is not closed with `{close}`");
                self.report_len(Class::Syntax, start, i, open.len(), message);
                break;
            };
            let embedded = text.trim() != &text[i..end + close.len()];
            self.expression(
                &text[i + open.len()..end],
                start + i + open.len(),
                place,
                embedded,
            );
            from = end + close.len();
        }
    }
}

type Cached = (core_syntax::SyntaxNode, usize, Rc<Vec<Finding>>);

thread_local! {
    static LAST: RefCell<Option<Cached>> = const { RefCell::new(None) };
}

/// Every expression problem of the file, found once for the file read last.
pub fn findings(spec: &'static Spec, file: &FileContext<'_>) -> Rc<Vec<Finding>> {
    let key = std::ptr::from_ref(spec) as usize;
    LAST.with_borrow_mut(|last| {
        if let Some((root, k, found)) = last
            && *k == key
            && root == file.root
        {
            return found.clone();
        }
        let mut run = Run {
            spec,
            out: Vec::new(),
        };
        for leaf in leaves(file) {
            let text = &file.text[leaf.range];
            let place = spec.place(&leaf.path);
            if text.contains(spec.open) || place.is_some_and(|p| p.implicit) {
                run.scalar(text, usize::from(leaf.range.start()), place);
            }
        }
        let found = Rc::new(run.out);
        *last = Some((file.root.clone(), key, found.clone()));
        found
    })
}

/// Reports the findings of `rule` in the file.
pub fn check(
    spec: &'static Spec,
    rule: &str,
    file: &FileContext<'_>,
    ctx: &mut crate::RuleContext<'_>,
) {
    for finding in findings(spec, file).iter().filter(|f| f.rule == rule) {
        ctx.report(finding.range, finding.message.clone());
    }
}
