//! Directory rules about the names a unit of files (the files of one directory,
//! [`crate::DirectoryScope`]) declares, uses, needs and provides, over a grammar of blocks (a head
//! word, labels, a body of `#key-value` members). The language fills a [`Names`] with its words
//! and declares its rules with [`names_rule!`]; the checks and the facts are written once.
//! - `Unused`: a declared name no file uses ([`Declare`]). `labels`: heads whose labels name the
//!   declaration (`head.label.label`), `members`: heads whose members do (`head.key`), `alias`
//!   with `alias_member`: heads whose block with that member declares `label.value`, `rename`:
//!   `(head, prefix)` for the first part of the address, `reference` and `reference_member` of
//!   [`Names`]: kinds of a name and of the accesses after it (the address is the name and the next
//!   access, `two_parts` heads take two).
//! - `MissingProvider`: a name used by a block that no member of a nested block provides
//!   ([`Provider`]). `use_heads`: heads whose first label gives the name up to the first
//!   `separator`, unless the member `use_member` names it; `name_heads`: heads whose first label
//!   is the name; `provide_head` and `provide_block`: the head of the block and of the block inside
//!   it whose members provide names; `exempt`: names nothing provides; `entry_key`: a provided
//!   entry that is a mapping must have this key (reported in its own file).
//! - `MissingSetting`: no block with head `Setting::head` has the member `Setting::name`;
//!   reported on each such block, or at the start of the file called `anchor` when the unit has
//!   none.
//!
//! The message has `{name}`.
//!
//! concept: module-names
//! what: Names a directory's files declare, use, need and provide: unused declarations, unprovided names, a missing setting.
//! tools: tflint terraform_unused_declarations, terraform_required_providers, terraform_required_version
//! reads: path:module_names::Names

use crate::directory_facts::{ANCHOR, DECLARED, NEEDED, PROVIDED, USED, missing, unused};
use crate::{
    DirectoryScope, ProjectContext, ProjectFacts, Rule, RuleContext, RuleMeta, Scope,
    rule::FileContext,
};
use core_syntax::{Grammar, SyntaxElement, SyntaxKind, SyntaxNode, TextRange};
use std::collections::BTreeSet;
use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq)]
pub enum Check {
    Unused,
    MissingProvider,
    MissingSetting,
}

/// What names a declaration.
pub struct Declare {
    pub labels: &'static [&'static str],
    pub members: &'static [&'static str],
    pub alias: &'static [&'static str],
    pub alias_member: &'static str,
    pub rename: &'static [(&'static str, &'static str)],
    pub two_parts: &'static [&'static str],
}

/// What a block uses and what provides it.
pub struct Provider {
    pub use_heads: &'static [&'static str],
    pub name_heads: &'static [&'static str],
    pub use_member: &'static str,
    pub separator: &'static str,
    pub provide_head: &'static str,
    pub provide_block: &'static str,
    pub exempt: &'static [&'static str],
    pub entry_key: &'static str,
}

/// A setting some block of the unit must have.
pub struct Setting {
    pub head: &'static str,
    pub name: &'static str,
    pub anchor: &'static str,
}

/// The words of a language for the three checks, as plain data.
pub struct Names {
    pub scope: DirectoryScope,
    /// Kind of the declaring nodes (top-level ones only) and of the block's body.
    pub block: &'static str,
    pub body: &'static str,
    /// The slots of the head word and of the labels.
    pub head: &'static str,
    pub labels: &'static str,
    pub reference: &'static str,
    pub reference_member: &'static str,
    pub declare: Declare,
    pub provider: Provider,
    pub setting: Setting,
}

/// A directory rule over [`Names`].
pub struct ModuleNames {
    meta: &'static RuleMeta,
    check: Check,
    message: &'static str,
    words: &'static Names,
    kinds: OnceLock<Option<Kinds>>,
}

impl ModuleNames {
    pub const fn new(
        meta: &'static RuleMeta,
        check: Check,
        message: &'static str,
        words: &'static Names,
    ) -> Self {
        ModuleNames {
            meta,
            check,
            message,
            words,
            kinds: OnceLock::new(),
        }
    }
}

/// Declares a static directory rule: the metadata like `rule!`, the check, the message and the
/// [`Names`] it reads.
#[macro_export]
macro_rules! names_rule {
    (
        $vis:vis $name:ident {
            id: $id:expr,
            category: $category:ident,
            default: $level:ident,
            description: $description:expr,
            example: $example:expr,
            check: $check:ident,
            message: $message:expr,
            names: $names:expr,
        }
    ) => {
        $vis static $name: $crate::module_names::ModuleNames =
            $crate::module_names::ModuleNames::new(
                {
                    static META: $crate::RuleMeta = $crate::RuleMeta {
                        id: $id,
                        category: $crate::Category::$category,
                        description: $description,
                        example: $example,
                        default_level: $crate::rule::Level::$level,
                        fix: None,
                        requires: &[],
                        uses: &[],
                        options: &[],
                    };
                    &META
                },
                $crate::module_names::Check::$check,
                $message,
                $names,
            );
    };
}

/// The grammar's kinds the words name.
struct Kinds {
    block: SyntaxKind,
    body: SyntaxKind,
    reference: Option<SyntaxKind>,
    reference_member: Option<SyntaxKind>,
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
                let w = self.words;
                Some(Kinds {
                    block: grammar.kind_by_name(w.block)?,
                    body: grammar.kind_by_name(w.body)?,
                    reference: grammar.kind_by_name(w.reference),
                    reference_member: grammar.kind_by_name(w.reference_member),
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
        let e = grammar.slot(block, self.words.head)?;
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
            let take = if self.words.declare.two_parts.contains(&head.as_str()) {
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
            .declare
            .rename
            .iter()
            .find(|(h, _)| *h == head)
            .map_or(head, |(_, p)| p)
    }

    fn collect_unused(&self, file: &FileContext<'_>, kinds: &Kinds, facts: &mut ProjectFacts) {
        for name in self.references(file.root, kinds) {
            facts.add(TextRange::default(), name, USED);
        }
        if self.words.scope.is_merged(file.path) {
            return;
        }
        let (w, grammar) = (&self.words.declare, file.grammar);
        for block in self.top_blocks(file.root, kinds) {
            let Some((head, head_range)) = self.head(grammar, &block) else {
                continue;
            };
            let labels = labels(grammar, &block, self.words.labels);
            if w.labels.contains(&head.as_str())
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
            if w.members.contains(&head.as_str()) {
                for (name, member) in &members {
                    let at = grammar
                        .slot(member, "key")
                        .map_or(member.text_range(), |k| element_range(&k));
                    facts.add(at, format!("{}.{name}", self.rename(&head)), DECLARED);
                }
            }
            if w.alias.contains(&head.as_str())
                && let Some(alias) = self.member_value(grammar, &members, w.alias_member)
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
        let (w, grammar) = (&self.words.provider, file.grammar);
        let labels = labels(grammar, block, self.words.labels);
        let (first, _) = labels.first()?;
        if w.name_heads.contains(&head) {
            return Some(first.clone());
        }
        if !w.use_heads.contains(&head) {
            return None;
        }
        let members = self.members(grammar, block, kinds);
        let given = self
            .member_value(grammar, &members, w.use_member)
            .and_then(|v| match v {
                SyntaxElement::Node(n) => {
                    n.descendants().find(|d| Some(d.kind()) == kinds.reference)
                }
                SyntaxElement::Token(_) => None,
            });
        match given {
            Some(reference) => Some(reference.text().to_string().trim().to_string()),
            None => first.split(w.separator).next().map(str::to_string),
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
                        .is_some_and(|h| h.0 == self.words.provider.provide_block)
            })
            .flat_map(|inner| self.members(grammar, &inner, kinds))
            .collect()
    }

    fn collect_provider(&self, file: &FileContext<'_>, kinds: &Kinds, facts: &mut ProjectFacts) {
        if self.words.scope.is_merged(file.path) {
            return;
        }
        let w = &self.words.provider;
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
                .filter(|n| !w.exempt.contains(&n.as_str()))
            {
                facts.add(at, name, NEEDED);
            }
        }
    }

    fn collect_setting(&self, file: &FileContext<'_>, kinds: &Kinds, facts: &mut ProjectFacts) {
        let w = &self.words.setting;
        if self.words.scope.is_merged(file.path) {
            return;
        }
        if file
            .path
            .file_name()
            .is_some_and(|n| n.to_str() == Some(w.anchor))
        {
            facts.add(TextRange::default(), w.name, ANCHOR);
        }
        for block in self.top_blocks(file.root, kinds) {
            let Some((head, at)) = self.head(file.grammar, &block) else {
                continue;
            };
            if head == w.head {
                let has = self
                    .members(file.grammar, &block, kinds)
                    .iter()
                    .any(|(n, _)| n == w.name);
                facts.add(at, w.name, if has { PROVIDED } else { NEEDED });
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
        Scope::Directory(self.words.scope)
    }

    /// A provided entry that is a mapping without the key `entry_key`, in its own file.
    fn check_file(&self, file: &FileContext<'_>, ctx: &mut RuleContext<'_>) {
        let w = &self.words.provider;
        if self.check != Check::MissingProvider
            || w.entry_key.is_empty()
            || self.words.scope.is_merged(file.path)
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
            Check::Unused => unused(facts, ctx, self.message),
            Check::MissingProvider | Check::MissingSetting => missing(facts, ctx, self.message),
        }
    }
}
