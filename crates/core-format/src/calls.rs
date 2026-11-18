//! The `[call]`, `[simple]` and `[hug]` sections of a language's rules file: what a call's
//! arguments look like for the layout that hugs, keeps on a line or breaks them
//! ([`crate::shapes::call_arguments`]).
//!
//! `[call]`: `set NAME = text|text` names a list of callee texts (a dotted name without blanks);
//! a row `CALLEE / ARGS : EFFECT` says that a call whose callee and arguments match is printed
//! with the effect. `CALLEE` is `&`-joined conditions: `callee=@SET` or `callee=a|b`,
//! `node=Kind|Kind`, `parent=Kind|Kind`. `ARGS` is a comma list of argument shapes, one per
//! argument (a trailing `+` repeats the last shape one or more times): `*`, or `Kind|#tag|...`
//! followed by `&` conditions: `block` (the function's `body` is a `#block`), `params<=N`,
//! `params=N`, `callee=@SET` and `arg=SHAPE` (a call with exactly that one argument),
//! `own-line` or `own-line=Kind` (a multi-line literal, or the last child of that kind, that starts on the line of the code before it) and
//! `comments=none` (no comment next to the node) or `comments=none-inside` (no comment next to
//! any of its tokens). Effects: `flat` (the arguments stay on the call's line), `test` (the same,
//! and the parameters of a function argument stay on its line) and `hook` (the arguments stay on
//! the line, as part of the argument list).
//!
//! `[simple]` (Prettier's "simple call argument") and `[hug]`: `key = Kind|#tag[:ARG]|...` rows,
//! the first row naming a node deciding. Keys: `leaf`, `regex` (`Kind:N`: a leaf of a pattern of
//! at most N characters), `single_line` (a literal with expressions that has no line break),
//! `object`, `array`, `items` (every child simple, one level deeper), `members` (every child
//! simple), `operand` (the first child simple), `unary` (`Kind: op op`: the last child simple),
//! `call` (callee and arguments simple, only at the top); `composition = yes` (a function
//! composition breaks its arguments), `no_composition` (parent kinds), `short` (always short),
//! `short_binary`, `cast` (`Kind:TypeArgumentsKind`: short when the first child is and the last
//! has no type arguments), `not_last` (`First:Last`: a pair of arguments the last of which does
//! not hug), `numeric` and `signed_numeric` (the items of a concise array), `concise`.

use crate::FormatCtx;
use crate::comments;
use core_syntax::roles::{code_text, un};
use core_syntax::{Grammar, SyntaxKind, SyntaxNode};

/// What a matching `[call]` row asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    Flat,
    Test,
    Hook,
}

impl Effect {
    /// The arguments are the layout's, not a member chain's.
    pub fn is_inline(self) -> bool {
        self != Effect::Hook
    }
}

#[derive(Clone, Debug)]
enum Pick {
    Kind(SyntaxKind),
    Tag(&'static str),
}

impl Pick {
    fn parse(g: &Grammar, n: &'static str) -> Pick {
        match n.trim().strip_prefix('#') {
            Some(t) => Pick::Tag(t),
            None => Pick::Kind(kind(g, n)),
        }
    }

    fn matches(&self, g: &Grammar, k: SyntaxKind) -> bool {
        match self {
            Pick::Kind(p) => *p == k,
            Pick::Tag(t) => g.has_tag(k, t),
        }
    }
}

/// Callee texts.
type Names = Vec<&'static str>;

#[derive(Clone, Debug)]
enum Cond {
    Block,
    Params(bool, usize),
    Callee(Names),
    Arg(Box<Shape>),
    OwnLine(Option<SyntaxKind>),
    Comments(bool),
}

#[derive(Clone, Debug, Default)]
struct Shape {
    any: bool,
    picks: Vec<Pick>,
    conds: Vec<Cond>,
}

#[derive(Debug)]
struct Pattern {
    node: Vec<SyntaxKind>,
    parent: Vec<SyntaxKind>,
    callee: Option<Names>,
    args: Vec<Shape>,
    repeat: bool,
    effect: Effect,
}

/// What a `[simple]` or `[hug]` row says about the nodes it names.
#[derive(Debug)]
enum Rule {
    Leaf,
    Regex(usize),
    SingleLine,
    Object,
    Array,
    Items,
    Members,
    Operand,
    Unary(Vec<&'static str>),
    Call,
    Short,
    ShortBinary,
    Cast(SyntaxKind),
    NotLast(SyntaxKind),
    Numeric,
    SignedNumeric,
    Concise,
    NoComposition,
}

impl Rule {
    fn is_simple(&self) -> bool {
        matches!(
            self,
            Rule::Leaf
                | Rule::Regex(_)
                | Rule::SingleLine
                | Rule::Object
                | Rule::Array
                | Rule::Items
                | Rule::Members
                | Rule::Operand
                | Rule::Unary(_)
                | Rule::Call
        )
    }
}

/// The rows of `[call]`, `[simple]` and `[hug]`.
#[derive(Default)]
pub struct CallRules {
    patterns: Vec<Pattern>,
    rows: Vec<(Pick, Rule)>,
    composition: bool,
}

fn kind(g: &Grammar, n: &str) -> SyntaxKind {
    g.kind_by_name(n.trim())
        .unwrap_or_else(|| panic!("call rules: kind `{n}`"))
}

fn kinds(g: &Grammar, v: &str) -> Vec<SyntaxKind> {
    v.split('|').map(|n| kind(g, n)).collect()
}

/// The `[call]` rows being read: the grammar and the sets declared so far.
struct Parse<'a> {
    g: &'a Grammar,
    sets: Vec<(&'a str, Names)>,
}

impl Parse<'_> {
    fn names(&self, v: &'static str) -> Names {
        match v.strip_prefix('@') {
            Some(n) => match self.sets.iter().find(|(d, _)| *d == n) {
                Some((_, names)) => names.clone(),
                None => panic!("call: set `{n}`"),
            },
            None => v.split('|').map(str::trim).collect(),
        }
    }

    fn pattern(&self, l: &'static str) -> Pattern {
        let g = self.g;
        let (head, effect) = l.rsplit_once(':').unwrap_or_else(|| panic!("call: `{l}`"));
        let (callee_part, args_part) = head
            .split_once(" / ")
            .unwrap_or_else(|| panic!("call: `{l}`"));
        let mut p = Pattern {
            node: Vec::new(),
            parent: Vec::new(),
            callee: None,
            args: Vec::new(),
            repeat: false,
            effect: match effect.trim() {
                "flat" => Effect::Flat,
                "test" => Effect::Test,
                "hook" => Effect::Hook,
                e => panic!("call: effect `{e}`"),
            },
        };
        for c in callee_part.split('&').filter(|c| *c != "*") {
            match c.trim().split_once('=') {
                Some(("callee", v)) => p.callee = Some(self.names(v)),
                Some(("node", v)) => p.node = kinds(g, v),
                Some(("parent", v)) => p.parent = kinds(g, v),
                _ => panic!("call: `{c}`"),
            }
        }
        // (a comma in `arg=` conditions is not a separator: shapes nest only one level)
        let mut shapes: Vec<&str> = args_part.split(',').map(str::trim).collect();
        if let Some(last) = shapes.last_mut()
            && let Some(s) = last.strip_suffix('+')
        {
            *last = s;
            p.repeat = true;
        }
        p.args = shapes.into_iter().map(|s| self.shape(s)).collect();
        p
    }

    fn shape(&self, s: &'static str) -> Shape {
        let g = self.g;
        let mut parts = s.split('&');
        let head = parts.next().unwrap_or("*").trim();
        let mut shape = Shape {
            any: head == "*",
            ..Shape::default()
        };
        if !shape.any {
            shape.picks = head.split('|').map(|n| Pick::parse(g, n)).collect();
        }
        for c in parts {
            shape.conds.push(match c.trim() {
                "block" => Cond::Block,
                "own-line" => Cond::OwnLine(None),
                "comments=none" => Cond::Comments(false),
                "comments=none-inside" => Cond::Comments(true),
                c => match c.split_once('=') {
                    Some(("own-line", k)) => Cond::OwnLine(Some(kind(g, k))),
                    Some(("params", n)) => Cond::Params(false, n.parse().expect("params")),
                    Some(("callee", v)) => Cond::Callee(self.names(v)),
                    Some(("arg", v)) => Cond::Arg(Box::new(self.shape(v))),
                    _ => match c.strip_prefix("params<=") {
                        Some(n) => Cond::Params(true, n.parse().expect("params")),
                        None => panic!("call: condition `{c}`"),
                    },
                },
            });
        }
        shape
    }
}

impl CallRules {
    pub fn parse(g: &Grammar, text: &'static str) -> CallRules {
        let mut rules = CallRules::default();
        let (mut call, mut sets, mut inside) = (Vec::new(), Vec::new(), "");
        for l in text.lines().map(str::trim) {
            if let Some(n) = l.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                inside = n;
            } else if l.is_empty() || l.starts_with('#') {
                continue;
            } else if inside == "call" {
                match l.strip_prefix("set ") {
                    Some(r) => {
                        let (name, list) = r.split_once('=').expect("call: set");
                        sets.push((name.trim(), list.split('|').map(str::trim).collect()));
                    }
                    None => call.push(l),
                }
            } else if let Some((key, v)) = l.split_once('=')
                && matches!(inside, "simple" | "hug")
            {
                rules.rows_of(g, key.trim(), v);
            }
        }
        let p = Parse { g, sets };
        rules.patterns = call.into_iter().map(|l| p.pattern(l)).collect();
        rules
    }

    /// The rows of one `key = Kind|#tag[:ARG]|...` line.
    fn rows_of(&mut self, g: &Grammar, key: &str, v: &'static str) {
        if key == "composition" {
            self.composition = v.trim() == "yes";
            return;
        }
        for part in v.split('|') {
            let (n, arg) = part.split_once(':').unwrap_or((part, ""));
            let rule = match key {
                "leaf" => Rule::Leaf,
                "regex" => Rule::Regex(arg.trim().parse().expect("regex length")),
                "single_line" => Rule::SingleLine,
                "object" => Rule::Object,
                "array" => Rule::Array,
                "items" => Rule::Items,
                "members" => Rule::Members,
                "operand" => Rule::Operand,
                "unary" => Rule::Unary(arg.split_whitespace().collect()),
                "call" => Rule::Call,
                "short" => Rule::Short,
                "short_binary" => Rule::ShortBinary,
                "cast" => Rule::Cast(kind(g, arg)),
                "not_last" => Rule::NotLast(kind(g, arg)),
                "numeric" => Rule::Numeric,
                "signed_numeric" => Rule::SignedNumeric,
                "concise" => Rule::Concise,
                "no_composition" => Rule::NoComposition,
                k => panic!("call rules: key `{k}`"),
            };
            self.rows.push((Pick::parse(g, n), rule));
        }
    }

    /// The first row that names the kind `k` and satisfies `f`.
    fn find(&self, g: &Grammar, k: SyntaxKind, f: fn(&Rule) -> bool) -> Option<&Rule> {
        let hit = |(p, r): &&(Pick, Rule)| f(r) && p.matches(g, k);
        self.rows.iter().find(hit).map(|(_, r)| r)
    }

    fn is(&self, g: &Grammar, k: SyntaxKind, f: fn(&Rule) -> bool) -> bool {
        self.find(g, k, f).is_some()
    }

    /// The effect of the first `[call]` row that matches the call `node` with callee `callee`
    /// and arguments `args`.
    pub fn pattern(
        &self,
        cx: &FormatCtx<'_>,
        node: &SyntaxNode,
        callee: Option<&SyntaxNode>,
        args: &[SyntaxNode],
    ) -> Option<Effect> {
        if self.patterns.is_empty()
            || args.is_empty()
            || core_syntax::roles::has_own_token(node, "?.")
        {
            return None;
        }
        // (the text of the callee is made when a row needs it, once)
        let text: std::cell::OnceCell<Option<String>> = std::cell::OnceCell::new();
        let callee_text = || {
            text.get_or_init(|| callee.and_then(|c| callee_text(cx, c)))
                .as_deref()
        };
        // (the cheap tests first: kind, argument count, parent, callee; the shapes last)
        let kind = node.kind();
        self.patterns
            .iter()
            .find(|p| {
                let n = p.args.len();
                (p.node.is_empty() || p.node.contains(&kind))
                    && (args.len() == n || (p.repeat && args.len() > n))
                    && (p.parent.is_empty()
                        || node.parent().is_some_and(|x| p.parent.contains(&x.kind())))
                    && p.callee
                        .as_ref()
                        .is_none_or(|n| callee_text().is_some_and(|t| n.contains(&t)))
                    && args
                        .iter()
                        .enumerate()
                        .all(|(i, a)| self.shape_matches(cx, &p.args[i.min(n - 1)], a))
            })
            .map(|p| p.effect)
    }

    fn shape_matches(&self, cx: &FormatCtx<'_>, s: &Shape, arg: &SyntaxNode) -> bool {
        let n = un(arg);
        let g = cx.grammar;
        (s.any || s.picks.iter().any(|p| p.matches(g, n.kind())))
            && s.conds.iter().all(|c| match c {
                Cond::Block => g
                    .slot(&n, "body")
                    .is_some_and(|b| g.has_tag(b.kind(), "block")),
                Cond::Params(le, count) => {
                    let have = g
                        .slot(&n, "parameters")
                        .and_then(|p| p.into_node())
                        .map_or(0, |p| {
                            p.children()
                                .filter(|c| g.has_tag(c.kind(), "parameter"))
                                .count()
                        });
                    if *le { have <= *count } else { have == *count }
                }
                Cond::Callee(names) => {
                    let callee = g.slot(&n, "callee").and_then(|c| c.into_node());
                    let text = callee.and_then(|c| callee_text(cx, &c));
                    text.is_some_and(|t| names.contains(&t.as_str()))
                }
                Cond::Arg(shape) => {
                    let mut args = g.slots(&n, "arguments").filter_map(|e| e.into_node());
                    matches!((args.next(), args.next()), (Some(a), None) if self.shape_matches(cx, shape, &a))
                }
                Cond::OwnLine(None) => template_on_own_line(cx, &n, &n),
                Cond::OwnLine(Some(k)) => n
                    .last_child()
                    .filter(|l| l.kind() == *k)
                    .is_some_and(|l| template_on_own_line(cx, &n, &l)),
                Cond::Comments(inside) => {
                    !has_comment(cx, &n)
                        && (!inside
                            || !n
                                .descendants_with_tokens()
                                .filter_map(|e| e.into_token())
                                .any(|t| comments::token_has_comment(cx.grammar, &t)))
                }
            })
    }

    /// Prettier's `isSimpleCallArgument`.
    pub fn is_simple(&self, cx: &FormatCtx<'_>, n: &SyntaxNode, depth: u32) -> bool {
        let (g, n) = (cx.grammar, un(n));
        let simple_at = |c: &SyntaxNode, d: u32| self.is_simple(cx, c, d);
        let text = || n.text().to_string();
        let Some(rule) = self.find(g, n.kind(), Rule::is_simple) else {
            return false;
        };
        match rule {
            Rule::Regex(max) => text().rfind('/').is_some_and(|end| end <= max + 1),
            Rule::SingleLine => {
                !text().contains('\n') && n.children().all(|c| simple_at(&c, depth + 1))
            }
            Rule::Object => n.children().all(|p| {
                g.has_tag(p.kind(), "key-value")
                    && !p
                        .children_with_tokens()
                        .any(|e| e.as_token().is_some_and(|t| t.text() == "["))
                    && (p.children().count() == 1
                        || p.last_child().is_some_and(|v| simple_at(&v, depth + 1)))
            }),
            Rule::Array => n
                .children()
                .all(|c| g.has_tag(c.kind(), "hole") || simple_at(&c, depth + 1)),
            Rule::Items => n.children().all(|c| simple_at(&c, depth + 1)),
            Rule::Members => n.children().all(|c| simple_at(&c, depth)),
            Rule::Operand => n.first_child().is_some_and(|o| simple_at(&o, depth)),
            Rule::Unary(ops) => {
                let op = g.slot(&n, "operator").and_then(|e| e.into_token());
                op.is_some_and(|t| ops.contains(&t.text()))
                    && n.last_child().is_some_and(|a| simple_at(&a, depth))
            }
            // A call inside a call argument is not simple.
            Rule::Call => {
                let callee = g.slot(&n, "callee").and_then(|c| c.into_node());
                depth == 0
                    && callee.is_some_and(|c| simple_at(&c, depth))
                    && g.slots(&n, "arguments")
                        .filter_map(|e| e.into_node())
                        .all(|a| simple_at(&a, depth + 1))
            }
            _ => true,
        }
    }

    /// Prettier's `isHopefullyShortCallArgument`: an argument that is cheap to print before a
    /// hugged first argument.
    pub fn is_short(&self, cx: &FormatCtx<'_>, n: &SyntaxNode) -> bool {
        let n = un(n);
        match self.find(cx.grammar, n.kind(), |r| {
            matches!(r, Rule::Short | Rule::ShortBinary | Rule::Cast(_))
        }) {
            Some(Rule::Short) => true,
            Some(Rule::ShortBinary) => n.children().all(|c| self.is_short(cx, &c)),
            Some(Rule::Cast(targs)) => {
                n.first_child().is_some_and(|e| self.is_short(cx, &e))
                    && n.last_child()
                        .is_some_and(|t| t.children().all(|c| c.kind() != *targs))
            }
            _ => self.is_simple(cx, &n, 0),
        }
    }

    /// Does a call with several function arguments (or one that has a function argument) break
    /// its arguments (`composition = yes`)?
    pub fn composition(&self) -> bool {
        self.composition
    }

    /// Arguments of a call under a node of this kind are never a function composition.
    pub fn no_composition(&self, g: &Grammar, parent: SyntaxKind) -> bool {
        self.is(g, parent, |r| matches!(r, Rule::NoComposition))
    }

    /// An argument pair `(a, b)` whose last argument is not hugged.
    pub fn not_last(&self, g: &Grammar, first: SyntaxKind, last: SyntaxKind) -> bool {
        self.rows
            .iter()
            .any(|(p, r)| matches!(r, Rule::NotLast(l) if *l == last) && p.matches(g, first))
    }

    /// An array of two or more numbers, which prints as a fill.
    pub fn is_concise_array(&self, cx: &FormatCtx<'_>, n: &SyntaxNode) -> bool {
        let g = cx.grammar;
        if !self.is(g, n.kind(), |r| matches!(r, Rule::Concise)) {
            return false;
        }
        let items: Vec<SyntaxNode> = n.children().collect();
        items.len() > 1 && items.iter().all(|i| self.concise_item(cx, i))
    }

    /// A number (signed, without comments inside) with no line comment after it or its comma.
    fn concise_item(&self, cx: &FormatCtx<'_>, i: &SyntaxNode) -> bool {
        let g = cx.grammar;
        let line_comment = |t: &core_syntax::SyntaxToken| {
            comments::trailing_comments(g, t)
                .iter()
                .any(|c| c.kind == comments::CommentKind::Line)
        };
        let numeric = |n: &SyntaxNode| self.is(g, n.kind(), |r| matches!(r, Rule::Numeric));
        let number = numeric(i)
            || (self.is(g, i.kind(), |r| matches!(r, Rule::SignedNumeric))
                && i.last_child()
                    .is_some_and(|c| numeric(&c) && !has_comment(cx, &c))
                && !cx
                    .first_token(i)
                    .is_some_and(|t| comments::token_has_comment(g, &t)));
        number
            && !cx.last_token(i).is_some_and(|t| {
                line_comment(&t)
                    || t.next_token()
                        .is_some_and(|c| g.has_tag(c.kind(), "separator") && line_comment(&c))
            })
    }
}

/// The text of a callee that is a name or a dotted name (`it.only`); `None` for any other callee.
fn callee_text(cx: &FormatCtx<'_>, callee: &SyntaxNode) -> Option<String> {
    let c = un(callee);
    let g = cx.grammar;
    (g.has_tag(c.kind(), "reference") || g.has_tag(c.kind(), "member-access"))
        .then(|| code_text(&c))
}

/// A comment in front of the first token of `n` or after its last.
pub fn has_comment(cx: &FormatCtx<'_>, n: &SyntaxNode) -> bool {
    cx.first_token(n)
        .is_some_and(|t| !comments::leading_comments(cx.grammar, &t).is_empty())
        || cx
            .last_token(n)
            .is_some_and(|t| comments::has_trailing_comment(cx.grammar, &t))
}

/// A node whose literal `lit` (the node, or its template literal when it is tagged) has a line
/// break and starts on the line of the code before it.
pub fn template_on_own_line(cx: &FormatCtx<'_>, n: &SyntaxNode, lit: &SyntaxNode) -> bool {
    let (Some(first), Some(last)) = (cx.first_token(n), cx.last_token(n)) else {
        return false;
    };
    // (a node may own trivia after its last token: a line break there is not in the literal)
    let from = cx.first_token(lit).unwrap_or_else(|| first.clone());
    if !cx.has_line_break(lit, from.text_range().cover(last.text_range())) {
        return false;
    }
    let mut prev = first.prev_token();
    while let Some(t) = prev.take().filter(|t| cx.grammar.is_trivia(t.kind())) {
        if cx.grammar.has_tag(t.kind(), "newline") {
            return false;
        }
        prev = t.prev_token();
    }
    true
}
