//! F501-F509: a `"..." % rhs` whose format string and right-hand side do not fit. The format is
//! read as CPython's `%` operator does; which right-hand sides are one value, a sequence or a
//! mapping follows ruff. (`str.format` is in `dot_format.rs`.)

use super::model::Model;
use ruff_python_ast::{self as ast, Expr, Operator, UnaryOp};
use ruff_text_size::Ranged;

/// What a format string asks for.
#[derive(Default)]
struct Spec {
    /// The conversions and `*` that take the next positional value.
    positional: usize,
    /// The `%(name)s` names, once each, in order.
    keys: Vec<String>,
    star: bool,
}

enum Failure {
    Invalid(&'static str),
    Char(char),
}

fn parse(text: &str) -> Result<Spec, Failure> {
    let mut spec = Spec::default();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '%' || chars.next_if_eq(&'%').is_some() {
            continue;
        }
        let mut named = false;
        if chars.next_if_eq(&'(').is_some() {
            let (mut depth, mut key) = (1, String::new());
            loop {
                let Some(c) = chars.next() else {
                    return Err(Failure::Invalid("incomplete format key"));
                };
                depth += i32::from(c == '(') - i32::from(c == ')');
                if depth == 0 {
                    break;
                }
                key.push(c);
            }
            named = true;
            if !spec.keys.contains(&key) {
                spec.keys.push(key);
            }
        }
        while chars.next_if(|c| "#0- +".contains(*c)).is_some() {}
        let mut stars = 0;
        for part in 0..2 {
            if part == 1 && chars.next_if_eq(&'.').is_none() {
                break;
            }
            if chars.next_if_eq(&'*').is_some() {
                stars += 1;
            } else {
                while chars.next_if(char::is_ascii_digit).is_some() {}
            }
        }
        while chars.next_if(|c| "hlL".contains(*c)).is_some() {}
        match chars.next() {
            None => return Err(Failure::Invalid("incomplete format")),
            Some(c) if !"diouxXeEfFgGcrsa".contains(c) => return Err(Failure::Char(c)),
            Some(_) => {}
        }
        spec.star |= stars > 0;
        spec.positional += stars + usize::from(!named);
    }
    Ok(spec)
}

/// An expression that is one value, never a tuple (ruff's reading).
fn single(e: &Expr) -> bool {
    let collection = |e: &Expr| {
        matches!(
            e,
            Expr::List(_)
                | Expr::Set(_)
                | Expr::Dict(_)
                | Expr::ListComp(_)
                | Expr::SetComp(_)
                | Expr::DictComp(_)
        )
    };
    match e {
        Expr::StringLiteral(_)
        | Expr::BytesLiteral(_)
        | Expr::NumberLiteral(_)
        | Expr::BooleanLiteral(_)
        | Expr::NoneLiteral(_)
        | Expr::EllipsisLiteral(_)
        | Expr::FString(_) => true,
        Expr::Generator(_) => true,
        _ if collection(e) => true,
        Expr::UnaryOp(u) => u.op == UnaryOp::Not || single(&u.operand),
        Expr::BinOp(b) => match b.op {
            Operator::MatMult => false,
            Operator::Mod if matches!(*b.left, Expr::StringLiteral(_)) => true,
            Operator::Mult | Operator::BitOr if collection(&b.left) || collection(&b.right) => {
                false
            }
            _ => single(&b.left) && single(&b.right),
        },
        Expr::BoolOp(b) => b.values.iter().all(single),
        Expr::If(i) => single(&i.body) && single(&i.orelse),
        Expr::Named(n) => single(&n.value),
        _ => false,
    }
}

enum Rhs<'a> {
    Tuple(&'a ast::ExprTuple),
    /// The string keys, when every key is one.
    Dict(Option<Vec<&'a str>>),
    Sequence,
    Single,
    Unknown,
}

fn classify(mut e: &Expr) -> Rhs<'_> {
    while let Expr::Named(n) = e {
        e = &n.value;
    }
    match e {
        Expr::Tuple(t) => Rhs::Tuple(t),
        Expr::Dict(d) => Rhs::Dict(
            d.items
                .iter()
                .map(|i| match i.key.as_ref() {
                    Some(Expr::StringLiteral(s)) => Some(s.value.to_str()),
                    _ => None,
                })
                .collect(),
        ),
        Expr::DictComp(_) => Rhs::Dict(None),
        Expr::List(_)
        | Expr::Set(_)
        | Expr::ListComp(_)
        | Expr::SetComp(_)
        | Expr::Generator(_) => Rhs::Sequence,
        _ if single(e) => Rhs::Single,
        _ => Rhs::Unknown,
    }
}

pub fn binop(m: &mut Model<'_>, b: &ast::ExprBinOp) {
    let (Operator::Mod, Expr::StringLiteral(s)) = (b.op, &*b.left) else {
        return;
    };
    let range = b.range();
    let spec = match parse(s.value.to_str()) {
        Ok(spec) => spec,
        Err(Failure::Invalid(why)) => return m.hit("percent-format-invalid-format", range, why),
        Err(Failure::Char(c)) => {
            return m.hit("percent-format-unsupported-format-character", range, c);
        }
    };
    let (named, pos) = (!spec.keys.is_empty(), spec.positional);
    let count = |m: &mut Model<'_>, got: usize| {
        if pos != got {
            m.hit(
                "percent-format-positional-count-mismatch",
                range,
                format!("{pos} placeholder(s) but {got} substitution(s)"),
            );
        }
    };
    if named && pos > 0 {
        m.hit("percent-format-mixed-positional-and-named", range, "");
    }
    match classify(&b.right) {
        Rhs::Tuple(_) if named => m.hit("percent-format-expected-mapping", range, ""),
        Rhs::Tuple(t) => {
            if !t.elts.iter().any(Expr::is_starred_expr) {
                count(m, t.elts.len());
            }
        }
        Rhs::Sequence if named => m.hit("percent-format-expected-mapping", range, ""),
        Rhs::Sequence => count(m, 1),
        Rhs::Dict(keys) => {
            if named || pos == 0 {
                if let Some(keys) = keys {
                    let unused: Vec<_> = keys
                        .iter()
                        .filter(|k| !spec.keys.iter().any(|s| s == *k))
                        .collect();
                    let mut missing: Vec<_> = spec
                        .keys
                        .iter()
                        .filter(|k| !keys.contains(&k.as_str()))
                        .collect();
                    missing.sort();
                    if !unused.is_empty() {
                        m.hit("percent-format-extra-named-arguments", range, join(unused));
                    }
                    if !missing.is_empty() {
                        m.hit("percent-format-missing-argument", range, join(missing));
                    }
                }
                if !named {
                    count(m, 1);
                }
            } else {
                if pos > 1 {
                    m.hit("percent-format-expected-sequence", range, "");
                }
                count(m, 1);
            }
            if spec.star {
                m.hit("percent-format-star-requires-sequence", range, "");
            }
        }
        Rhs::Single if !named => count(m, 1),
        Rhs::Unknown if !named && pos == 0 => count(m, 1),
        Rhs::Single | Rhs::Unknown => {}
    }
}

fn join<T: std::fmt::Display>(items: Vec<T>) -> String {
    items
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}
