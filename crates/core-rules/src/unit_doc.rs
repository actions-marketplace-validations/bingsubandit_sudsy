//! The family `unit-doc-comment`: a directory whose files have no doc comment before their
//! namespace clause (the Go package comment) is reported once, at one file of the directory. The
//! rule is a directory rule ([`crate::DirectoryScope`]): the files of the directory are read
//! even when only some of them are linted, and the rule is silent when one of them cannot be
//! read, so the answer never depends on which files were asked for. Params:
//! - `members`: the file name suffixes of the files of a directory (`.go`);
//! - the doc comment as of `missing-doc-comment` ([`crate::doc_run::Docs`]);
//! - `trigger-except`: file name suffixes of files that count for the doc comment but never
//!   cause the report (`_test.go`);
//! - `detached-prefix`: a file whose comment of this form (`Package `, then the name) is parted
//!   from the clause by a blank line neither has a doc comment nor causes the report;
//! - `message`.
//!
//! The report is at the namespace keyword of the first file by name.
//!
//! concept: unit-doc-comment
//! what: A directory whose files have no doc comment before their namespace clause is reported once.
//! tools: revive package-comments
//! rules: missing-package-comment
//! reads: family:unit-doc-comment, tag:namespace

use crate::directory_facts::{ANCHOR, NEEDED, PROVIDED, first_anchor};
use crate::doc_run::Docs;
use crate::tree::is_code;
use crate::{
    DirectoryScope, Family, FileContext, Params, ProjectContext, ProjectFacts, Rule, RuleMeta,
    Scope,
};

pub const FAMILY: Family = Family {
    name: "unit-doc-comment",
    build,
};

struct UnitDoc {
    meta: &'static RuleMeta,
    message: String,
    scope: DirectoryScope,
    docs: Docs,
    except: Vec<String>,
    detached: String,
}

fn words(text: Option<String>) -> Vec<String> {
    text.unwrap_or_default()
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

fn leak(words: Vec<String>) -> &'static [&'static str] {
    let list: Vec<&'static str> = words
        .into_iter()
        .map(|w| &*Box::leak(w.into_boxed_str()))
        .collect();
    Box::leak(list.into_boxed_slice())
}

fn build(meta: &'static RuleMeta, mut p: Params) -> Result<Box<dyn Rule>, String> {
    let message = p.text("message")?;
    let members = leak(words(Some(p.text("members")?)));
    let except = words(p.optional("trigger-except"));
    let detached = p.optional("detached-prefix").unwrap_or_default();
    let docs = Docs::read(&mut p)?;
    p.done()?;
    Ok(Box::new(UnitDoc {
        meta,
        message,
        scope: DirectoryScope {
            members,
            opaque: &[],
            merged: &[],
        },
        docs,
        except,
        detached,
    }))
}

impl Rule for UnitDoc {
    fn meta(&self) -> &'static RuleMeta {
        self.meta
    }

    fn scope(&self) -> Scope {
        Scope::Directory(self.scope)
    }

    fn collect(&self, file: &FileContext<'_>, facts: &mut ProjectFacts) {
        let g = file.grammar;
        let Some(clause) = file
            .root
            .children()
            .find(|c| g.has_tag(c.kind(), "namespace"))
        else {
            return;
        };
        let Some(keyword) = clause
            .descendants_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| is_code(t, g))
        else {
            return;
        };
        let path = crate::slot_path::SlotPath::parse("name").unwrap_or_else(|_| unreachable!());
        let name = path
            .find(g, &clause, file.text)
            .map_or_else(String::new, |f| f.text.to_string());
        facts.add(keyword.text_range(), name.clone(), ANCHOR);
        let named = file
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        let test = self.except.iter().any(|e| named.ends_with(e.as_str()));
        let detached = !self.detached.is_empty()
            && (self
                .docs
                .detached(g, &clause, &format!("{}{name} ", self.detached)))
            .is_some();
        if self.docs.exists(g, &clause) {
            facts.add(keyword.text_range(), name, PROVIDED);
        } else if !test && !detached {
            facts.add(keyword.text_range(), name, NEEDED);
        }
    }

    fn finish(&self, facts: &ProjectFacts, ctx: &mut ProjectContext<'_>) {
        first_anchor(facts, ctx, &self.message);
    }
}
