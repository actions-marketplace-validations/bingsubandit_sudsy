//! Project scope: rules gather facts from each file, then report across
//! files once all files have been seen.
//!
//! concept: none (infrastructure: project scope)

use crate::diagnostic::{Diagnostic, Severity};
use crate::options::RuleOptions;
use crate::rule::RuleMeta;
use core_syntax::TextRange;
use std::path::{Path, PathBuf};

/// What a rule's project phase looks at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// Every file of the project; only at `analysis: deep`.
    Project,
    /// The files of one directory, the unit a language judges as a whole; at every depth, and the
    /// rule collects nothing when it is off.
    Directory(DirectoryScope),
}

/// The files that make up a directory's unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DirectoryScope {
    /// File name suffixes of the unit's files (`.tf`).
    pub members: &'static [&'static str],
    /// Suffixes of files that belong to the unit but that Sudsy does not read: a directory with
    /// one has facts of unknown completeness.
    pub opaque: &'static [&'static str],
    /// Suffixes of files whose declarations are merged into the others (`_override.tf`).
    pub merged: &'static [&'static str],
}

impl DirectoryScope {
    fn named(path: &Path, suffixes: &[&str]) -> bool {
        path.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| suffixes.iter().any(|s| n.ends_with(s)))
    }

    pub fn is_member(&self, path: &Path) -> bool {
        Self::named(path, self.members)
    }

    pub fn is_opaque(&self, path: &Path) -> bool {
        Self::named(path, self.opaque)
    }

    /// A member whose declarations are merged into the others (an override file).
    pub fn is_merged(&self, path: &Path) -> bool {
        Self::named(path, self.merged)
    }
}

/// One fact a rule gathered from a file. Facts are plain strings so that they can be cached with
/// the file's result and compared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fact {
    pub rule: &'static str,
    pub file: PathBuf,
    pub range: TextRange,
    pub key: String,
    pub value: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProjectFacts {
    facts: Vec<Fact>,
    /// The (rule, file) pairs whose `collect` ran: a file that is missing here left no facts
    /// because it could not be read (it did not parse), not because it has none.
    collected: Vec<(&'static str, PathBuf)>,
    current: Option<(&'static str, PathBuf)>,
}

impl ProjectFacts {
    /// Records a fact for the rule and file being collected.
    pub fn add(&mut self, range: TextRange, key: impl Into<String>, value: impl Into<String>) {
        let (rule, file) = self
            .current
            .clone()
            .expect("facts are added from `Rule::collect` only");
        self.facts.push(Fact {
            rule,
            file,
            range,
            key: key.into(),
            value: value.into(),
        });
    }

    /// Facts that were gathered elsewhere (a plugin's project phase gets them back this way).
    pub fn from_facts(facts: Vec<Fact>) -> Self {
        ProjectFacts {
            facts,
            collected: Vec::new(),
            current: None,
        }
    }

    pub fn facts(&self) -> &[Fact] {
        &self.facts
    }

    pub(crate) fn begin(&mut self, rule: &'static str, file: &Path) {
        self.collected.push((rule, file.to_path_buf()));
        self.current = Some((rule, file.to_path_buf()));
    }

    pub(crate) fn end(&mut self) {
        self.current = None;
    }

    /// The facts of one rule, in the order the files were added.
    pub fn of_rule<'a>(&'a self, id: &'a str) -> impl Iterator<Item = &'a Fact> {
        self.facts.iter().filter(move |f| f.rule == id)
    }

    pub fn extend(&mut self, other: &ProjectFacts) {
        self.facts.extend(other.facts.iter().cloned());
        self.collected.extend(other.collected.iter().cloned());
    }

    /// Whether `rule` collected from `file`.
    pub fn collected(&self, rule: &str, file: &Path) -> bool {
        self.collected.iter().any(|(r, f)| *r == rule && f == file)
    }

    pub fn is_empty(&self) -> bool {
        self.facts.is_empty()
    }
}

pub struct ProjectContext<'a> {
    pub options: &'a RuleOptions,
    pub(crate) meta: &'static RuleMeta,
    /// A directory rule: the unit's files, in the order of their facts.
    pub files: &'a [PathBuf],
    /// A directory rule: every file of the unit was read and collected, so a rule that reports
    /// what is absent from the unit may do so. False otherwise: such a rule stays silent.
    pub complete: bool,
    pub(crate) severity: Severity,
    pub(crate) out: &'a mut Vec<(PathBuf, Diagnostic)>,
}

impl ProjectContext<'_> {
    pub fn rule_id(&self) -> &'static str {
        self.meta.id
    }

    pub fn report(
        &mut self,
        file: &Path,
        range: TextRange,
        message: impl Into<String>,
    ) -> &mut Diagnostic {
        self.out.push((
            file.to_path_buf(),
            Diagnostic {
                rule: self.meta.id.to_string(),
                category: self.meta.category,
                severity: self.severity,
                message: message.into(),
                range,
                related: Vec::new(),
                fixes: Vec::new(),
            },
        ));
        &mut self.out.last_mut().expect("just pushed").1
    }
}
