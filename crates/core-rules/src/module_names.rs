//! The `module-names` family: directory rules about the names a unit of files (the files of one
//! directory, [`crate::DirectoryScope`]) declares, uses, needs and provides, over a grammar of
//! blocks (a head word, labels, a body of `#key-value` members). The language supplies the words
//! as params; the checks and the facts are written once. Common params: `block` (kind of the
//! declaring nodes, top-level ones only), `body` (kind of the block's body), `head` and `labels`
//! (the slots of the head word and of the labels), `members`, `opaque` and `merged` (the
//! directory scope: suffixes of the unit's files, of files not read, of files merged into the
//! others, which give uses but no declarations and no needs), and `check`:
//! - `unused`: a declared name no file uses. `declare_labels`: heads whose labels name the
//!   declaration (`head.label.label`), `declare_members`: heads whose members do (`head.key`),
//!   `declare_alias` with `alias_member`: heads whose block with a member `alias_member` declares
//!   `label.value`, `rename`: `head=prefix` pairs for the first part of the address, `reference`
//!   and `reference_member`: kinds of a name and of the accesses after it (the address is the name
//!   and the next access, `two_parts` heads take two).
//! - `missing-provider`: a name used by a block that no member of a nested block provides.
//!   `use_heads`: heads whose first label gives the name up to the first `separator`, unless the
//!   member `use_member` names it; `name_heads`: heads whose first label is the name;
//!   `provide_head` and `provide_block`: the head of the block and of the block inside it whose
//!   members provide names; `exempt`: names nothing provides; `entry_key`: a provided entry that
//!   is a mapping must have this key (reported in its own file).
//! - `missing-setting`: no block with head `setting_head` has the member `setting`; reported on
//!   each such block, or at the start of the file called `anchor` when the unit has none.
//!
//! The message has `{name}`.
//!
//! concept: module-names
//! what: Names a directory's files declare, use, need and provide: unused declarations, unprovided names, a missing setting.
//! tools: tflint terraform_unused_declarations, terraform_required_providers, terraform_required_version

use crate::directory_facts::{ANCHOR, DECLARED, NEEDED, PROVIDED, USED, missing, unused};
use crate::{
    DirectoryScope, Family, Params, ProjectContext, ProjectFacts, Rule, RuleContext, RuleMeta,
    Scope, rule::FileContext,
};
use core_syntax::{Grammar, SyntaxElement, SyntaxKind, SyntaxNode, TextRange};
use std::collections::BTreeSet;
use std::sync::OnceLock;

pub const FAMILY: Family = Family {
    name: "module-names",
    build,
};

#[derive(Clone, Copy, PartialEq)]
enum Check {
    Unused,
    MissingProvider,
    MissingSetting,
}

/// The params as sets of words, read once.
#[derive(Default)]
struct Words {
    block: String,
    body: String,
    head: String,
    labels: String,
    declare_labels: Vec<String>,
    declare_members: Vec<String>,
    declare_alias: Vec<String>,
    alias_member: String,
    rename: Vec<(String, String)>,
    reference: String,
    reference_member: String,
    two_parts: Vec<String>,
    use_heads: Vec<String>,
    name_heads: Vec<String>,
    use_member: String,
    separator: String,
    provide_head: String,
    provide_block: String,
    exempt: Vec<String>,
    entry_key: String,
    setting_head: String,
    setting: String,
    anchor: String,
}

struct ModuleNames {
    meta: &'static RuleMeta,
    check: Check,
    message: String,
    scope: DirectoryScope,
    words: Words,
    kinds: OnceLock<Option<Kinds>>,
}

/// The grammar's kinds the words name.
struct Kinds {
    block: SyntaxKind,
    body: SyntaxKind,
    reference: Option<SyntaxKind>,
    reference_member: Option<SyntaxKind>,
}

fn set(p: &mut Params, key: &str) -> Vec<String> {
    p.optional(key)
        .map(|v| v.split_whitespace().map(str::to_string).collect())
        .unwrap_or_default()
}

fn text(p: &mut Params, key: &str) -> String {
    p.optional(key).unwrap_or_default()
}

fn leaked(words: Vec<String>) -> &'static [&'static str] {
    let words: Vec<&'static str> = words
        .into_iter()
        .map(|w| &*Box::leak(w.into_boxed_str()))
        .collect();
    Box::leak(words.into_boxed_slice())
}

fn build(meta: &'static RuleMeta, mut p: Params) -> Result<Box<dyn Rule>, String> {
    let check = p.choice(
        "check",
        true,
        &[
            ("unused", Check::Unused),
            ("missing-provider", Check::MissingProvider),
            ("missing-setting", Check::MissingSetting),
        ],
    )?;
    let scope = DirectoryScope {
        members: leaked(set(&mut p, "members")),
        opaque: leaked(set(&mut p, "opaque")),
        merged: leaked(set(&mut p, "merged")),
    };
    let rename = set(&mut p, "rename")
        .iter()
        .filter_map(|pair| pair.split_once('='))
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect();
    let words = Words {
        block: p.text("block")?,
        body: p.text("body")?,
        head: p.text("head")?,
        labels: p.text("labels")?,
        declare_labels: set(&mut p, "declare_labels"),
        declare_members: set(&mut p, "declare_members"),
        declare_alias: set(&mut p, "declare_alias"),
        alias_member: text(&mut p, "alias_member"),
        rename,
        reference: text(&mut p, "reference"),
        reference_member: text(&mut p, "reference_member"),
        two_parts: set(&mut p, "two_parts"),
        use_heads: set(&mut p, "use_heads"),
        name_heads: set(&mut p, "name_heads"),
        use_member: text(&mut p, "use_member"),
        separator: text(&mut p, "separator"),
        provide_head: text(&mut p, "provide_head"),
        provide_block: text(&mut p, "provide_block"),
        exempt: set(&mut p, "exempt"),
        entry_key: text(&mut p, "entry_key"),
        setting_head: text(&mut p, "setting_head"),
        setting: text(&mut p, "setting"),
        anchor: text(&mut p, "anchor"),
    };
    let message = p.text("message")?;
    p.done()?;
    Ok(Box::new(ModuleNames {
        meta,
        check,
        message,
        scope,
        words,
        kinds: OnceLock::new(),
    }))
}

fn element_text(e: &SyntaxElement) -> String {
    match e {
        SyntaxElement::Token(t) => t.text().to_string(),
        SyntaxElement::Node(n) => n.text().to_string(),
    }
    .trim()
    .trim_matches('"')
    .to_string()
}

fn element_range(e: &SyntaxElement) -> TextRange {
    e.text_range()
}

/// The names of the labels of a block with the range of each.
fn labels(grammar: &Grammar, block: &SyntaxNode, slot: &str) -> Vec<(String, TextRange)> {
    grammar
        .slots(block, slot)
        .map(|l| (element_text(&l), element_range(&l)))
        .collect()
}

impl ModuleNames {
    fn kinds(&self, grammar: &Grammar) -> Option<&Kinds> {
        self.kinds
            .get_or_init(|| {
                let w = &self.words;
                Some(Kinds {
                    block: grammar.kind_by_name(&w.block)?,
                    body: grammar.kind_by_name(&w.body)?,
                    reference: grammar.kind_by_name(&w.reference),
                    reference_member: grammar.kind_by_name(&w.reference_member),
                })
            })
            .as_ref()
    }

    /// The blocks at the top of the file.
    fn top_blocks(&self, root: &SyntaxNode, kinds: &Kinds) -> Vec<SyntaxNode> {
        root.descendants()
            .filter(|n| {
                n.kind() == kinds.block
                    && n.ancestors().filter(|a| a.kind() == kinds.block).count() == 1
            })
            .collect()
    }

    /// The head word of a block and where it is.
    fn head(&self, grammar: &Grammar, block: &SyntaxNode) -> Option<(String, TextRange)> {
        let e = grammar.slot(block, &self.words.head)?;
        Some((element_text(&e), element_range(&e)))
    }

    /// The members of a block's body: name, node.
    fn members(
        &self,
        grammar: &Grammar,
        block: &SyntaxNode,
        kinds: &Kinds,
    ) -> Vec<(String, SyntaxNode)> {
        block
            .children()
            .find(|c| c.kind() == kinds.body)
            .into_iter()
            .flat_map(|b| b.children().collect::<Vec<_>>())
            .filter(|m| grammar.has_tag(m.kind(), "key-value"))
            .filter_map(|m| Some((element_text(&grammar.slot(&m, "key")?), m)))
            .collect()
    }

    fn member_value(
        &self,
        grammar: &Grammar,
        members: &[(String, SyntaxNode)],
        name: &str,
    ) -> Option<SyntaxElement> {
        let (_, m) = members.iter().find(|(n, _)| n == name)?;
        grammar.slot(m, "value")
    }

    /// The addresses a file refers to: a name and its next access (or two).
    fn references(&self, root: &SyntaxNode, kinds: &Kinds) -> BTreeSet<String> {
        let (Some(reference), Some(member)) = (kinds.reference, kinds.reference_member) else {
            return BTreeSet::new();
        };
        let mut found = BTreeSet::new();
        for node in root.descendants().filter(|n| n.kind() == reference) {
            let head = node.text().to_string().trim().to_string();
            let mut parts = std::iter::successors(node.next_sibling(), |n| n.next_sibling())
                .take_while(|n| n.kind() == member)
                .map(|n| {
                    n.text()
                        .to_string()
                        .trim()
                        .trim_start_matches('.')
                        .trim()
                        .to_string()
                });
            let take = if self.words.two_parts.contains(&head) {
                2
            } else {
                1
            };
            let mut address = head;
            for _ in 0..take {
                match parts.next() {
                    Some(part) => address = format!("{address}.{part}"),
                    None => return found,
                }
            }
            found.insert(address);
        }
        found
    }

    fn rename<'a>(&'a self, head: &'a str) -> &'a str {
        self.words
            .rename
            .iter()
            .find(|(h, _)| h == head)
            .map_or(head, |(_, p)| p.as_str())
    }

    fn collect_unused(&self, file: &FileContext<'_>, kinds: &Kinds, facts: &mut ProjectFacts) {
        for name in self.references(file.root, kinds) {
            facts.add(TextRange::default(), name, USED);
        }
        if self.scope.is_merged(file.path) {
            return;
        }
        let (w, grammar) = (&self.words, file.grammar);
        for block in self.top_blocks(file.root, kinds) {
            let Some((head, head_range)) = self.head(grammar, &block) else {
                continue;
            };
            let labels = labels(grammar, &block, &w.labels);
            if w.declare_labels.contains(&head)
                && let Some((_, last)) = labels.last()
            {
                let address = labels
                    .iter()
                    .map(|l| l.0.as_str())
                    .collect::<Vec<_>>()
                    .join(".");
                facts.add(*last, format!("{}.{address}", self.rename(&head)), DECLARED);
            }
            let members = self.members(grammar, &block, kinds);
            if w.declare_members.contains(&head) {
                for (name, member) in &members {
                    let at = grammar
                        .slot(member, "key")
                        .map_or(member.text_range(), |k| element_range(&k));
                    facts.add(at, format!("{}.{name}", self.rename(&head)), DECLARED);
                }
            }
            if w.declare_alias.contains(&head)
                && let Some(alias) = self.member_value(grammar, &members, &w.alias_member)
                && let Some((name, _)) = labels.first()
            {
                facts.add(
                    head_range,
                    format!("{name}.{}", element_text(&alias)),
                    DECLARED,
                );
            }
        }
    }

    /// The name a block uses.
    fn used_name(
        &self,
        file: &FileContext<'_>,
        kinds: &Kinds,
        block: &SyntaxNode,
        head: &str,
    ) -> Option<String> {
        let (w, grammar) = (&self.words, file.grammar);
        let labels = labels(grammar, block, &w.labels);
        let (first, _) = labels.first()?;
        if w.name_heads.iter().any(|h| h == head) {
            return Some(first.clone());
        }
        if !w.use_heads.iter().any(|h| h == head) {
            return None;
        }
        let members = self.members(grammar, block, kinds);
        let given = self
            .member_value(grammar, &members, &w.use_member)
            .and_then(|v| match v {
                SyntaxElement::Node(n) => {
                    n.descendants().find(|d| Some(d.kind()) == kinds.reference)
                }
                SyntaxElement::Token(_) => None,
            });
        match given {
            Some(reference) => Some(reference.text().to_string().trim().to_string()),
            None => first.split(w.separator.as_str()).next().map(str::to_string),
        }
    }

    /// The `provide_block` entries inside a block: name, node.
    fn provided(
        &self,
        file: &FileContext<'_>,
        kinds: &Kinds,
        block: &SyntaxNode,
    ) -> Vec<(String, SyntaxNode)> {
        let grammar = file.grammar;
        block
            .children()
            .find(|c| c.kind() == kinds.body)
            .into_iter()
            .flat_map(|b| b.children().collect::<Vec<_>>())
            .filter(|n| {
                n.kind() == kinds.block
                    && self
                        .head(grammar, n)
                        .is_some_and(|h| h.0 == self.words.provide_block)
            })
            .flat_map(|inner| self.members(grammar, &inner, kinds))
            .collect()
    }

    fn collect_provider(&self, file: &FileContext<'_>, kinds: &Kinds, facts: &mut ProjectFacts) {
        if self.scope.is_merged(file.path) {
            return;
        }
        let w = &self.words;
        for block in self.top_blocks(file.root, kinds) {
            let Some((head, at)) = self.head(file.grammar, &block) else {
                continue;
            };
            if head == w.provide_head {
                for (name, _) in self.provided(file, kinds, &block) {
                    facts.add(at, name, PROVIDED);
                }
            } else if let Some(name) = self
                .used_name(file, kinds, &block, &head)
                .filter(|n| !w.exempt.contains(n))
            {
                facts.add(at, name, NEEDED);
            }
        }
    }

    fn collect_setting(&self, file: &FileContext<'_>, kinds: &Kinds, facts: &mut ProjectFacts) {
        let w = &self.words;
        if self.scope.is_merged(file.path) {
            return;
        }
        if file
            .path
            .file_name()
            .is_some_and(|n| n.to_str() == Some(w.anchor.as_str()))
        {
            facts.add(TextRange::default(), &w.setting, ANCHOR);
        }
        for block in self.top_blocks(file.root, kinds) {
            let Some((head, at)) = self.head(file.grammar, &block) else {
                continue;
            };
            if head == w.setting_head {
                let has = self
                    .members(file.grammar, &block, kinds)
                    .iter()
                    .any(|(n, _)| *n == w.setting);
                facts.add(at, &w.setting, if has { PROVIDED } else { NEEDED });
            }
        }
    }
}

impl Rule for ModuleNames {
    fn meta(&self) -> &'static RuleMeta {
        self.meta
    }

    fn bodies(&self) -> bool {
        false
    }

    fn scope(&self) -> Scope {
        Scope::Directory(self.scope)
    }

    /// A provided entry that is a mapping without the key `entry_key`, in its own file.
    fn check_file(&self, file: &FileContext<'_>, ctx: &mut RuleContext<'_>) {
        let w = &self.words;
        if self.check != Check::MissingProvider
            || w.entry_key.is_empty()
            || self.scope.is_merged(file.path)
        {
            return;
        }
        let Some(kinds) = self.kinds(file.grammar) else {
            return;
        };
        let grammar = file.grammar;
        for block in self.top_blocks(file.root, kinds) {
            if self
                .head(grammar, &block)
                .is_none_or(|h| h.0 != w.provide_head)
            {
                continue;
            }
            for (name, entry) in self.provided(file, kinds, &block) {
                let mapping = entry
                    .descendants()
                    .find(|n| grammar.has_tag(n.kind(), "mapping"));
                let keyed = mapping.is_none_or(|m| {
                    m.children()
                        .filter(|e| grammar.has_tag(e.kind(), "key-value"))
                        .any(|e| {
                            grammar
                                .slot(&e, "key")
                                .is_some_and(|k| element_text(&k) == w.entry_key)
                        })
                });
                if !keyed {
                    let at = grammar
                        .slot(&entry, "key")
                        .map_or(entry.text_range(), |k| element_range(&k));
                    ctx.report(at, self.message.replace("{name}", &name));
                }
            }
        }
    }

    fn collect(&self, file: &FileContext<'_>, facts: &mut ProjectFacts) {
        let Some(kinds) = self.kinds(file.grammar) else {
            return;
        };
        match self.check {
            Check::Unused => self.collect_unused(file, kinds, facts),
            Check::MissingProvider => self.collect_provider(file, kinds, facts),
            Check::MissingSetting => self.collect_setting(file, kinds, facts),
        }
    }

    fn finish(&self, facts: &ProjectFacts, ctx: &mut ProjectContext<'_>) {
        match self.check {
            Check::Unused => unused(facts, ctx, &self.message),
            Check::MissingProvider | Check::MissingSetting => missing(facts, ctx, &self.message),
        }
    }
}
