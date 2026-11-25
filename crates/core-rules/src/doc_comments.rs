//! Doc comments: the `/** ... */` block comment before a declaration ([`leading_doc`]) and the
//! family `missing-doc-comment`, which reports a visible declaration without one. Params:
//! - `<group>`: declaration rows ([`crate::decl_rows`]) of what needs a doc comment (a group of
//!   rows has a name of its own; `outer.<group>` lists the modifier keywords of which each
//!   enclosing type declaration must have one, empty: no condition);
//! - `visible`: the modifier keywords that make a declaration visible (a declaration with none of
//!   them is not reported), `implicit-public-in`: the keywords of the type declarations whose
//!   members are visible without a modifier (Java `interface @interface`);
//! - `getters` and `setters`: name prefixes of property accessors (`get is`, `set`), which the
//!   option `property_accessors` leaves out: a getter has no parameter and one `#return`
//!   statement, a setter one parameter and one expression statement;
//! - `message` with `{name}`;
//! - `doc-start`: the prefixes of a doc comment (`///`), a comment element (token or node) that
//!   starts with one is the doc comment; without it a `/** */` block comment is;
//! - `report-at: name`: the report is at the name of the declaration (the row's path), not at its
//!   first token;
//! - `visible-with`: slots (`explicit_interface_specifier`) that make a declaration visible without
//!   a modifier, `skip`: modifier keywords (`partial`) of the declarations left out;
//! - `top-level-visible`: a declaration outside every type is visible without a modifier;
//! - `visible-by-case`: a name is visible when it starts with an uppercase letter, a method also
//!   when its receiver type does (`receiver.<group>`: the slot with the receiver,
//!   `receiver-token.<group>`: the kind of the token that names the type);
//! - `skip-names.<group>`: names left out, `sortable.<group>`: names left out when the file
//!   declares all of them for the same receiver type, `skip-namespaces`: namespaces whose files
//!   are left alone;
//! - `doc-lines`, `doc-skip`, `group-comment`, `group-open` (see [`crate::doc_run`]): the doc is
//!   a run of line comments right above, and the comment of a group of declarations;
//! - `once.<group>`: only the first declaration of a group without a doc comment is reported.
//!
//! A declaration inside a function or an object creation is not reported. The option
//! `allowed_annotations` (a list of names, `*` matches any run of characters) leaves out a
//! declaration with one of them. The report is at the first token of the declaration.
//!
//! concept: missing-doc-comment
//! what: A visible declaration has a doc comment before it.
//! tools: Checkstyle MissingJavadocMethod, MissingJavadocType
//! rules: missing-doc-comment
//! reads: family:missing-doc-comment, tag:modifiers, tag:annotation, tag:parameter-list

use crate::decl_rows::{Row, annotation_names, glob, has_modifier};
use crate::doc_run::Docs;
use crate::slot_path::{Found, SlotPath};
use crate::tree::is_code;
use crate::{Family, Params, Rule, RuleContext, RuleMeta};
use core_syntax::{Grammar, SyntaxNode, SyntaxToken};
use rowan::NodeOrToken;

pub const FAMILY: Family = Family {
    name: "missing-doc-comment",
    build,
};

/// The doc comment (a block comment that starts with the opening delimiter and one more of its
/// last character: `/**`) among the comments before the first code token of `node`.
pub fn leading_doc(node: &SyntaxNode, g: &Grammar) -> Option<SyntaxToken> {
    let (open, close) = g.comments.block.as_ref()?;
    let last = open.chars().last()?;
    let mut found = None;
    for t in node
        .descendants_with_tokens()
        .filter_map(|e| e.into_token())
    {
        if is_code(&t, g) {
            break;
        }
        let text = t.text();
        if g.has_tag(t.kind(), "comment")
            && text.starts_with(open.as_str())
            && text[open.len()..].starts_with(last)
            && text.len() > open.len() + close.len()
        {
            found = Some(t);
        }
    }
    found
}

/// The text of the doc comment before the first code token of `holder`: with `starts`, the
/// first comment element (a token, or a node such as a structured doc comment) that starts with
/// one of them; without, the `/** */` block comment ([`leading_doc`]).
pub fn leading_doc_text(holder: &SyntaxNode, g: &Grammar, starts: &[String]) -> Option<String> {
    if starts.is_empty() {
        return leading_doc(holder, g).map(|t| t.text().to_string());
    }
    for e in holder.descendants_with_tokens() {
        if let Some(t) = e.as_token() {
            if is_code(t, g) {
                return None;
            }
        } else if let Some(n) = e.as_node()
            && g.has_tag(n.kind(), "comment")
        {
            let text = n.text().to_string();
            if starts.iter().any(|s| text.starts_with(s.as_str())) {
                return Some(text);
            }
        }
    }
    None
}

pub(crate) struct Group {
    rows: Vec<Row>,
    outer: Vec<String>,
    /// `receiver.<group>` and `receiver-token.<group>`: the slot with the receiver of a method
    /// and the kind of token that names its type; the type must be visible too.
    receiver: String,
    receiver_token: String,
    /// `skip-names.<group>`: names of declarations that are left out.
    skip_names: Vec<String>,
    /// `sortable.<group>`: names left out when the receiver type has all of them in the file.
    sortable: Vec<String>,
    /// `articles.<group>`: words a doc comment may start with before the name.
    pub(crate) articles: Vec<String>,
    /// `once.<group>`: only the first declaration of a group of declarations is reported.
    once: bool,
    /// `form-in-group.<group>`: the doc comment of a group in parentheses is the declaration's.
    pub(crate) form_in_group: bool,
    /// `prefix.<group>`: the text a doc comment starts with before the name.
    pub(crate) prefix: String,
    /// `detached.<group>`: a comment of the right form that a blank line parts from the
    /// declaration is reported.
    pub(crate) detached: bool,
}

struct Missing {
    meta: &'static RuleMeta,
    message: String,
    targets: Targets,
    getters: Vec<String>,
    setters: Vec<String>,
    doc_start: Vec<String>,
    docs: Docs,
}

/// The declarations a family looks at: the rows of its groups, which of them are visible, and
/// the facts of the language about names (`visible-by-case`, `skip-names`, `receiver`, ...).
pub(crate) struct Targets {
    groups: Vec<Group>,
    pub(crate) vis: Visibility,
    pub(crate) tags: Vec<&'static str>,
    pub(crate) report_name: bool,
    by_case: bool,
    skip_namespaces: Vec<String>,
}

/// A declaration of a group: the node with the modifiers, and its name.
pub(crate) struct Hit<'g, 't> {
    pub(crate) holder: SyntaxNode,
    pub(crate) name: Found<'t>,
    pub(crate) group: &'g Group,
}

/// Which declarations are visible: the params `visible`, `implicit-public-in`,
/// `top-level-visible`, `visible-with` and `skip` of the families that read doc comments.
pub(crate) struct Visibility {
    visible: Vec<String>,
    implicit: Vec<String>,
    top_visible: bool,
    visible_with: Vec<String>,
    skip: Vec<String>,
}

pub(crate) fn words(text: &str) -> Vec<String> {
    text.split_whitespace().map(str::to_string).collect()
}

fn build(meta: &'static RuleMeta, mut p: Params) -> Result<Box<dyn Rule>, String> {
    let message = p.text("message")?;
    let getters = words(&p.optional("getters").unwrap_or_default());
    let setters = words(&p.optional("setters").unwrap_or_default());
    let doc_start = words(&p.optional("doc-start").unwrap_or_default());
    let docs = Docs::read(&mut p)?;
    let targets = Targets::read(&mut p)?;
    Ok(Box::new(Missing {
        meta,
        message,
        targets,
        getters,
        setters,
        doc_start,
        docs,
    }))
}

impl Targets {
    /// Reads `visible` and the other params of [`Visibility`], `report-at`, `visible-by-case`,
    /// `skip-namespaces` and every param left: a declaration group (`<group>: rows`) and its
    /// `<key>.<group>` params.
    pub(crate) fn read(p: &mut Params) -> Result<Targets, String> {
        let vis = Visibility::read(p)?;
        let report_name = p.choice(
            "report-at",
            false,
            &[("first-token", false), ("name", true)],
        )?;
        let by_case = p.flag("visible-by-case")?;
        let skip_namespaces = words(&p.optional("skip-namespaces").unwrap_or_default());
        let mut qualified = Vec::new();
        let mut groups = Vec::new();
        for (key, value) in p.rest() {
            if let Some((what, group)) = key.split_once('.') {
                qualified.push((what.to_string(), group.to_string(), value));
                continue;
            }
            let rows = (value.split('|'))
                .map(|alt| Row::parse(&key, alt))
                .collect::<Result<Vec<_>, _>>()?;
            groups.push((key, Group::new(rows)));
        }
        for (what, name, value) in qualified {
            let g = groups.iter_mut().find(|(k, _)| *k == name);
            let g = &mut g
                .ok_or_else(|| format!("param `{what}.{name}` names no group"))?
                .1;
            match what.as_str() {
                "outer" => g.outer = words(&value),
                "receiver" => g.receiver = value,
                "receiver-token" => g.receiver_token = value,
                "skip-names" => g.skip_names = words(&value),
                "sortable" => g.sortable = words(&value),
                "articles" => g.articles = words(&value),
                "once" => g.once = value == "true",
                "form-in-group" => g.form_in_group = value == "true",
                "prefix" => g.prefix = value,
                "detached" => g.detached = value == "true",
                _ => return Err(format!("unknown param `{what}.{name}`")),
            }
        }
        let groups: Vec<Group> = groups.into_iter().map(|(_, g)| g).collect();
        let mut tags: Vec<&'static str> = Vec::new();
        for r in groups.iter().flat_map(|g| &g.rows) {
            if !tags.contains(&r.tag.as_str()) {
                tags.push(Box::leak(r.tag.clone().into_boxed_str()));
            }
        }
        Ok(Targets {
            groups,
            vis,
            tags,
            report_name,
            by_case,
            skip_namespaces,
        })
    }

    /// The declaration `node` is, when the family looks at it: it matches a row, is visible, is not
    /// inside a function, and its group's conditions on the name and the receiver hold.
    pub(crate) fn select<'g, 't>(
        &'g self,
        g: &Grammar,
        node: &SyntaxNode,
        text: &'t str,
    ) -> Option<Hit<'g, 't>> {
        let (group, row) = self
            .groups
            .iter()
            .flat_map(|gr| gr.rows.iter().map(move |r| (gr, r)))
            .filter(|(_, r)| r.matches(g, node))
            .reduce(|a, b| {
                if b.1.conditions() > a.1.conditions() {
                    b
                } else {
                    a
                }
            })?;
        let holder = holder(g, node);
        if !self.vis.visible_declaration(g, node, &holder)
            || (node.ancestors().skip(1))
                .any(|a| g.has_tag(a.kind(), "function") || g.has_tag(a.kind(), "call"))
            || (!group.outer.is_empty() && !self.vis.enclosing_visible(g, node, &group.outer))
            || self.skipped_file(g, node, text)
        {
            return None;
        }
        let default = SlotPath::parse("name").unwrap_or_else(|_| unreachable!());
        let name = (row.path.as_ref().unwrap_or(&default)).find(g, node, text)?;
        if self.by_case && !exported(&name.text) {
            return None;
        }
        if group.skip_names.iter().any(|n| *n == name.text) {
            return None;
        }
        if !group.receiver.is_empty() {
            let receiver = receiver_type(g, node, group);
            if self.by_case && !receiver.as_deref().is_some_and(exported) {
                return None;
            }
            let sortable = !group.sortable.is_empty()
                && group.sortable.iter().any(|n| *n == name.text)
                && receiver.is_some_and(|r| has_all(g, node, text, group, &r));
            if sortable {
                return None;
            }
        }
        Some(Hit {
            holder,
            name,
            group,
        })
    }

    /// Is the file of `node` one the family skips (the package name is in `skip-namespaces`)?
    fn skipped_file(&self, g: &Grammar, node: &SyntaxNode, text: &str) -> bool {
        if self.skip_namespaces.is_empty() {
            return false;
        }
        let Some(root) = node.ancestors().last() else {
            return false;
        };
        let name = root.children().find(|c| g.has_tag(c.kind(), "namespace"));
        let path = SlotPath::parse("name").unwrap_or_else(|_| unreachable!());
        let name = name.and_then(|n| path.find(g, &n, text).map(|f| f.text.to_string()));
        name.is_some_and(|n| self.skip_namespaces.contains(&n))
    }

    /// Every other declaration of the same group (the siblings before `node`) that the family
    /// would look at and `lacks` a doc comment.
    pub(crate) fn earlier_sibling(
        &self,
        g: &Grammar,
        node: &SyntaxNode,
        text: &str,
        lacks: impl Fn(&Hit<'_, '_>) -> bool,
    ) -> bool {
        node.siblings(rowan::Direction::Prev)
            .skip(1)
            .filter_map(|s| self.select(g, &s, text).map(|h| (s, h)))
            .any(|(_, h)| lacks(&h))
    }
}

impl Group {
    fn new(rows: Vec<Row>) -> Group {
        Group {
            rows,
            outer: Vec::new(),
            receiver: String::new(),
            receiver_token: String::new(),
            skip_names: Vec::new(),
            sortable: Vec::new(),
            articles: Vec::new(),
            once: false,
            form_in_group: false,
            prefix: String::new(),
            detached: false,
        }
    }

    /// Is only the first declaration of a group of declarations without a doc comment reported?
    pub(crate) fn once(&self) -> bool {
        self.once
    }
}

/// Is the name exported: does it start with an uppercase letter?
pub(crate) fn exported(name: &str) -> bool {
    name.chars().next().is_some_and(char::is_uppercase)
}

/// Does the file declare every name of `sortable` for the type `receiver`?
fn has_all(g: &Grammar, node: &SyntaxNode, text: &str, group: &Group, receiver: &str) -> bool {
    let Some(root) = node.ancestors().last() else {
        return false;
    };
    let name = SlotPath::parse("name").unwrap_or_else(|_| unreachable!());
    let mut found = Vec::new();
    for n in root.children() {
        if receiver_type(g, &n, group).as_deref() == Some(receiver)
            && let Some(f) = name.find(g, &n, text)
        {
            found.push(f.text.to_string());
        }
    }
    group.sortable.iter().all(|w| found.contains(w))
}

/// The name of the type in the receiver of a method: the first token of the group's receiver
/// token kind inside its receiver slot.
fn receiver_type(g: &Grammar, node: &SyntaxNode, group: &Group) -> Option<String> {
    let kind = g.kind_by_name(&group.receiver_token)?;
    let NodeOrToken::Node(n) = g.slot(node, &group.receiver)? else {
        return None;
    };
    (n.descendants_with_tokens())
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == kind)
        .map(|t| t.text().to_string())
}

/// The node with the modifiers of a declaration: `node`, or its parent when the parent holds them.
pub fn holder(g: &Grammar, node: &SyntaxNode) -> SyntaxNode {
    match node.parent() {
        Some(p) if p.children().any(|c| g.has_tag(c.kind(), "modifiers")) => p,
        _ => node.clone(),
    }
}

impl Visibility {
    pub(crate) fn read(p: &mut Params) -> Result<Visibility, String> {
        Ok(Visibility {
            visible: words(&p.optional("visible").unwrap_or_default()),
            implicit: words(&p.optional("implicit-public-in").unwrap_or_default()),
            top_visible: p.flag("top-level-visible")?,
            visible_with: words(&p.optional("visible-with").unwrap_or_default()),
            skip: words(&p.optional("skip").unwrap_or_default()),
        })
    }

    /// Is the declaration in `holder` (inside `node`'s enclosing declarations) visible without a modifier?
    fn implicit(&self, g: &Grammar, node: &SyntaxNode) -> bool {
        let Some(outer) = node
            .ancestors()
            .skip(1)
            .find(|a| g.has_tag(a.kind(), "type-declaration"))
        else {
            return false;
        };
        let keyword = outer
            .descendants_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| is_code(t, g));
        let listed = |t: &SyntaxToken| self.implicit.iter().any(|w| w == t.text());
        keyword.as_ref().is_some_and(listed)
            || outer
                .children_with_tokens()
                .filter_map(|e| e.into_token())
                .any(|t| listed(&t))
    }

    /// Is the declaration outside every type (and function)?
    fn top_level(&self, g: &Grammar, node: &SyntaxNode) -> bool {
        self.top_visible
            && !node
                .ancestors()
                .skip(1)
                .any(|a| g.has_tag(a.kind(), "type-declaration"))
    }

    /// Is the declaration reported at all: not skipped, and visible?
    pub(crate) fn visible_declaration(
        &self,
        g: &Grammar,
        node: &SyntaxNode,
        holder: &SyntaxNode,
    ) -> bool {
        !(self.skip.iter()).any(|w| has_modifier(g, holder, w))
            && self.visible_by_modifier(g, node, holder)
    }

    pub(crate) fn words(&self) -> &[String] {
        &self.visible
    }

    fn visible_by_modifier(&self, g: &Grammar, node: &SyntaxNode, holder: &SyntaxNode) -> bool {
        (self.visible.iter()).any(|w| has_modifier(g, holder, w))
            || (self.implicit(g, node) && !has_modifier(g, holder, "private"))
            || (self.top_level(g, node) && !has_modifier(g, holder, "private"))
            || (self.visible_with.iter()).any(|slot| g.slot(node, slot).is_some())
    }

    /// Is every enclosing type declaration visible by one of `outer` (or implicitly)?
    pub(crate) fn enclosing_visible(
        &self,
        g: &Grammar,
        node: &SyntaxNode,
        outer: &[String],
    ) -> bool {
        node.ancestors()
            .skip(1)
            .filter(|a| g.has_tag(a.kind(), "type-declaration"))
            .all(|a| {
                let h = holder(g, &a);
                outer.iter().any(|w| has_modifier(g, &h, w))
                    || ((self.implicit(g, &a) || self.top_level(g, &a))
                        && !has_modifier(g, &h, "private"))
            })
    }
}

impl Missing {
    fn accessor(&self, g: &Grammar, node: &SyntaxNode, name: &str) -> bool {
        let prefixed = |words: &[String]| {
            (words.iter()).any(|w| {
                (name.strip_prefix(w.as_str())).is_some_and(|r| r.starts_with(char::is_uppercase))
            })
        };
        let (getter, setter) = (prefixed(&self.getters), prefixed(&self.setters));
        if !getter && !setter {
            return false;
        }
        let params = (node.children())
            .find(|c| g.has_tag(c.kind(), "parameter-list"))
            .map_or(0, |l| {
                l.children()
                    .filter(|c| g.has_tag(c.kind(), "parameter"))
                    .count()
            });
        let Some(body) = node.children().find(|c| g.has_tag(c.kind(), "block")) else {
            return false;
        };
        let mut statements = body.children().filter(|c| g.has_tag(c.kind(), "statement"));
        let (Some(only), None) = (statements.next(), statements.next()) else {
            return false;
        };
        if getter {
            params == 0 && g.has_tag(only.kind(), "return")
        } else {
            params == 1 && g.has_tag(only.kind(), "expression-statement")
        }
    }
}

impl Missing {
    fn has_doc(&self, g: &Grammar, holder: &SyntaxNode) -> bool {
        if self.docs.lines() {
            self.docs.of(g, holder).is_some()
        } else {
            leading_doc_text(holder, g, &self.doc_start).is_some()
        }
    }
}

impl Rule for Missing {
    fn meta(&self) -> &'static RuleMeta {
        self.meta
    }

    fn bodies(&self) -> bool {
        false
    }

    fn tags(&self) -> &[&'static str] {
        &self.targets.tags
    }

    fn check(&self, node: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        let g = ctx.file.grammar;
        let text = ctx.file.text;
        let Some(hit) = self.targets.select(g, node, text) else {
            return;
        };
        let holder = hit.holder.clone();
        if self.has_doc(g, &holder) {
            return;
        }
        if hit.group.once()
            && self.targets.earlier_sibling(g, node, text, |h| {
                h.group.once() && !self.has_doc(g, &h.holder)
            })
        {
            return;
        }
        let allowed = ctx.options.list("allowed_annotations");
        if !allowed.is_empty()
            && annotation_names(g, &holder).iter().any(|n| {
                let simple = n.rsplit('.').next().unwrap_or(n);
                allowed.iter().any(|a| glob(a, n) || glob(a, simple))
            })
        {
            return;
        }
        let name = hit.name;
        if ctx.options.bool("property_accessors") && self.accessor(g, node, &name.text) {
            return;
        }
        if self.targets.report_name {
            ctx.report(name.range, self.message.replace("{name}", &name.text));
            return;
        }
        let Some(first) = holder
            .descendants_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| is_code(t, g))
        else {
            return;
        };
        ctx.report(
            first.text_range(),
            self.message.replace("{name}", &name.text),
        );
    }
}
