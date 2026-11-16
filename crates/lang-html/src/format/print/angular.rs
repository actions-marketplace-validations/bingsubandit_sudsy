//! Angular's template syntax in the printer: control flow blocks, `@let`, ICU messages, bindings,
//! microsyntax and pipes. Expressions are formatted by the engine as TypeScript (`Opts::embed`);
//! pipes, which are no TypeScript, are split off first (`pipes`).

use super::{P, s};
use crate::format::model::{N, T};
use core_format::EmbedMode;
use core_format::doc::{self, Doc};
use oxc_htmlish_parser::ast::{AngularAttribute, AttributeKind, NodeKind, TemplateBinding};
use oxc_htmlish_parser::{Allocator, Dialect, Parser};

mod pipes;

/// The blocks that continue a block of this name (Prettier's `ANGULAR_CONTROL_FLOW_BLOCK_WITH_PARAMETERS`):
/// the block does not close with `}` before one of them.
fn continues(name: &str, next: &str) -> bool {
    let chain = ["placeholder", "error", "loading"];
    match name {
        "if" | "else if" => matches!(next, "else" | "else if"),
        "for" => next == "empty",
        "defer" => chain.contains(&next),
        _ => chain.contains(&name) && chain.contains(&next),
    }
}

/// `a   b` as `a b`.
fn collapse(t: &str) -> String {
    t.split_whitespace().collect::<Vec<_>>().join(" ")
}

impl P<'_> {
    /// An expression formatted for `width`; `statements` reads `a; b` (an event handler).
    pub(super) fn ng_expression(
        &self,
        code: &str,
        width: usize,
        single_quotes: bool,
        statements: bool,
    ) -> Option<String> {
        let embed = self.o.embed?;
        let fmt = |c: &str| embed("ts", EmbedMode::Expression, c, width, single_quotes);
        if statements {
            let parts = pipes::split_statements(code);
            let out: Option<Vec<String>> = parts.iter().map(|p| pipes::format(p, &fmt)).collect();
            let out = out?;
            let flat = out.join("; ");
            return Some(if flat.len() <= width && !flat.contains('\n') {
                flat
            } else {
                out.join(";\n")
            });
        }
        pipes::format(code, &fmt)
    }

    /// `name="` formatted value `"` for the binding `name`; `None` keeps the attribute as written.
    pub(super) fn angular_attribute(&self, id: usize, name: &str, value: &str) -> Option<Doc> {
        let v = super::unescape_quotes(value);
        let (open, close) = (format!("{name}=\""), "\"");
        if name.starts_with('(') || name.starts_with("on-") {
            let f = |w: usize| self.ng_expression(&v, w, true, true);
            return self.ng_attr(id, &open, close, &f);
        }
        if name.starts_with(['[']) || name.starts_with("bind-") {
            let f = |w: usize| self.ng_expression(&v, w, true, false);
            return self.ng_attr(id, &open, close, &f);
        }
        if name.starts_with('*') {
            let f = |w: usize| self.microsyntax(&v, w);
            return self.ng_attr(id, &open, close, &f);
        }
        if name == "i18n" || name.starts_with("i18n-") {
            let words = collapse(&v);
            return Some(s(format!("{name}=\"{}\"", words.replace('"', "&quot;"))));
        }
        if v.contains("{{") {
            return self.attribute_interpolation(id, &open, &v);
        }
        None
    }

    fn ng_attr(
        &self,
        id: usize,
        open: &str,
        close: &str,
        run: &dyn Fn(usize) -> Option<String>,
    ) -> Option<Doc> {
        let run = |w: usize| run(w).map(|o| (o, false));
        self.attr_expression(id, open, close, &|o| o.replace('"', "&quot;"), &run)
    }

    /// `Hi {{name}}!`: the text kept, each interpolation a group that breaks inside its braces.
    fn attribute_interpolation(&self, id: usize, open: &str, v: &str) -> Option<Doc> {
        let width = self
            .o
            .width
            .saturating_sub((self.depth(id) + 2) * self.o.tab);
        let quoted = |t: &str| {
            doc::join(
                &doc::hardline(),
                t.split('\n').map(|l| s(l.replace('"', "&quot;"))),
            )
        };
        let mut parts = Vec::new();
        let mut rest = v;
        while let Some(i) = rest.find("{{") {
            let end = rest[i + 2..].find("}}")? + i + 2;
            parts.push(quoted(&rest[..i]));
            let code = rest[i + 2..end].trim();
            let body = if code.is_empty() {
                doc::line()
            } else {
                let out = self.ng_expression(code, width, true, false)?;
                doc::concat(vec![doc::line(), quoted(&out)])
            };
            parts.push(doc::group(doc::concat(vec![
                s("{{"),
                doc::indent(body),
                doc::line(),
                s("}}"),
            ])));
            rest = &rest[end + 2..];
        }
        parts.push(quoted(rest));
        Some(doc::concat(vec![
            s(open),
            doc::group(doc::concat(parts)),
            s("\""),
        ]))
    }

    /// The value of a structural directive (`let item of items; trackBy: f`), as Prettier prints
    /// its microsyntax; read with the parser's own reading of a start tag.
    fn microsyntax(&self, value: &str, width: usize) -> Option<String> {
        let quote = ['"', '\''].into_iter().find(|q| !value.contains(*q))?;
        let src = format!("<a *d={quote}{value}{quote}>");
        let allocator = Allocator::default();
        let root = Parser::new(&allocator, &src, Dialect::Angular)
            .parse()
            .ok()?;
        let element = root.nodes.iter().find_map(|n| match &n.kind {
            NodeKind::Element(e) => Some(e),
            _ => None,
        })?;
        let attribute = element.attributes.first()?;
        let AttributeKind::Angular(AngularAttribute::Template(Some(bindings))) = &attribute.kind
        else {
            return None;
        };
        let text = |sp: oxc_htmlish_parser::Span| &src[sp.start as usize..sp.end as usize];
        let expr = |sp| self.ng_expression(text(sp), width, true, false);
        let mut out = String::new();
        let (mut first_is_expression, mut previous_key): (bool, Option<&str>) = (false, None);
        // The directive's own empty first binding is not printed; an `as` that names the expression
        // before it is part of it.
        let items = bindings.iter().filter(|b| {
            !matches!(
                b,
                TemplateBinding::Expression {
                    key: None,
                    expression: None
                }
            )
        });
        let mut i = 0;
        for b in items {
            let alias = matches!(b, TemplateBinding::As { key: None, .. });
            if !alias {
                i += 1;
            }
            let i = i - 1;
            let mut piece = String::new();
            match b {
                TemplateBinding::Expression { key, expression } => {
                    let key = key.map(text);
                    if let Some(k) = key {
                        piece.push_str(k);
                    }
                    if let Some(e) = expression {
                        let colon = key.is_some() && {
                            let k = key.unwrap_or("");
                            !((i == 1 && k == "of")
                                || (first_is_expression
                                    && ((i == 1 && matches!(k, "then" | "else" | "as"))
                                        || ((i == 2 || i == 3)
                                            && (k == "track"
                                                || (k == "else"
                                                    && previous_key == Some("then")))))))
                        };
                        if key.is_some() {
                            piece.push_str(if colon { ": " } else { " " });
                        }
                        piece.push_str(&expr(*e)?);
                    }
                    if i == 0 {
                        first_is_expression = key.is_none();
                    }
                    previous_key = key;
                }
                TemplateBinding::Let { name, value } => {
                    piece.push_str(&format!("let {}", text(*name)));
                    if let Some(v) = value {
                        piece.push_str(&format!(" = {}", text(*v)));
                    }
                    previous_key = None;
                }
                TemplateBinding::As { key, alias } => {
                    if let Some(k) = key {
                        piece.push_str(&format!("{} ", text(*k)));
                    }
                    piece.push_str(&format!("as {}", text(*alias)));
                }
            }
            if alias {
                out.push(' ');
            } else if i > 0 {
                let of = matches!(b, TemplateBinding::Expression { key: Some(k), .. } if i == 1 && text(*k) == "of");
                out.push_str(if of { " " } else { "; " });
            }
            out.push_str(&piece);
        }
        Some(out)
    }

    /// The parameters of a block: expressions of `if`, `else if`, `for`, `switch` and `case`
    /// formatted, the rest of each parameter with its blanks collapsed.
    fn block_parameters(&self, n: &N<'_>, width: usize) -> String {
        let name = collapse(n.name.trim_start_matches('@'));
        let expr = |code: &str| {
            self.ng_expression(code.trim(), width, false, false)
                .unwrap_or_else(|| collapse(code))
        };
        let mut out = Vec::new();
        for (i, p) in n.params.iter().enumerate() {
            let p = p.trim();
            out.push(match (name.as_str(), i) {
                ("if" | "else if" | "switch" | "case", 0) => expr(p),
                ("for", _) => {
                    if let Some(rest) = p
                        .strip_prefix("track")
                        .filter(|r| r.starts_with(char::is_whitespace))
                    {
                        format!("track {}", expr(rest))
                    } else if let Some((item, list)) = p.split_once(" of ") {
                        format!("{} of {}", collapse(item), expr(list))
                    } else {
                        p.to_string()
                    }
                }
                _ => collapse(p),
            });
        }
        out.join("; ")
    }

    fn block_name(n: &N<'_>) -> String {
        collapse(n.name.trim_start_matches('@'))
    }

    /// Does the block print its `}`? Not when a block that continues it follows.
    fn closes(&self, id: usize) -> bool {
        let n = self.n(id);
        !n.next.is_some_and(|x| {
            self.n(x).t == T::NgBlock
                && continues(&Self::block_name(n), &Self::block_name(self.n(x)))
        })
    }

    /// `@name (parameters) {`, the content indented, `}`; the `}` of a block another follows
    /// moves to that block's line.
    pub(super) fn ng_block_doc(&self, id: usize) -> Doc {
        let n = self.n(id);
        let mut head = String::new();
        if let Some(p) = n.prev
            && self.n(p).t == T::NgBlock
            && !self.closes(p)
        {
            head.push_str("} ");
        }
        head.push('@');
        head.push_str(&Self::block_name(n));
        if !n.params.is_empty() {
            let width = self
                .o
                .width
                .saturating_sub((self.depth(id) + 1) * self.o.tab);
            head.push_str(&format!(" ({})", self.block_parameters(n, width)));
        }
        if n.self_closing {
            head.push(';');
            return s(head);
        }
        head.push_str(" {");
        let mut parts = vec![s(head)];
        let close = self.closes(id);
        if n.kids.is_empty() {
            if close {
                parts.push(s("}"));
            }
        } else {
            parts.push(doc::indent(doc::concat(vec![
                doc::hardline(),
                self.children(id),
            ])));
            if close {
                parts.extend([doc::hardline(), s("}")]);
            }
        }
        doc::group(doc::concat(parts))
    }

    /// `@let name = value;`
    pub(super) fn ng_let_doc(&self, id: usize) -> Doc {
        let n = self.n(id);
        let width = self
            .o
            .width
            .saturating_sub((self.depth(id) + 1) * self.o.tab);
        let value = self
            .ng_expression(n.value.trim(), width, false, false)
            .unwrap_or_else(|| n.value.trim().to_string());
        s(format!("@let {} = {};", n.name.trim(), value))
    }

    /// `{value, type, =0 {one} other {many}}`
    pub(super) fn icu_doc(&self, id: usize) -> Doc {
        let n = self.n(id);
        let mut head = vec![s(format!(
            "{}{}, {}",
            self.open_start(id),
            n.value.trim(),
            n.name.trim()
        ))];
        if !n.kids.is_empty() {
            let cases: Vec<Doc> = n.kids.iter().map(|&c| self.icu_case(c)).collect();
            head.push(s(","));
            head.push(doc::indent(doc::concat(vec![
                doc::line(),
                doc::join(&doc::line(), cases),
            ])));
        }
        head.push(doc::softline());
        head.push(s(self.close_end(id)));
        doc::group(doc::concat(head))
    }

    fn icu_case(&self, id: usize) -> Doc {
        let n = self.n(id);
        let mut body = Vec::new();
        for (i, &c) in n.kids.iter().enumerate() {
            let k = self.n(c);
            if k.t == T::Text && k.lead_sp {
                body.push(doc::line());
            }
            // An interpolation in a case stays as written.
            body.push(if k.t == T::Interp {
                s(&self.t.src[k.start..k.end])
            } else {
                self.print_child(c)
            });
            if k.t == T::Text && k.trail_sp && i + 1 < n.kids.len() {
                body.push(doc::line());
            }
        }
        doc::concat(vec![
            s(format!("{} {{", n.value.trim())),
            doc::group(doc::concat(vec![
                doc::indent(doc::concat(vec![doc::softline(), doc::concat(body)])),
                doc::softline(),
            ])),
            s("}"),
        ])
    }
}
