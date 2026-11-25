//! The symbol index: where a name is declared, answered as `Known`, `Absent` or `Unknown`. A
//! name is looked for in the project's files, then in the files the package locator names for it
//! (read on demand), or, in a language whose names are visible through imports, in the imported
//! files. `Absent` needs proof: every project file read and declaring nothing in disguise, and the
//! locator covering the name; wherever a fact is missing the answer is `Unknown`, and a rule
//! reports only on `Absent`.
//!
//! concept: symbol-index
//! what: Answers whether a type, function or member exists: known with its declaration, proved absent, or unknown.
//! tools: PHPStan class.notFound, buf IMPORT_USED
//! reads: file:symbols.rows

use crate::db::Db;
use crate::inputs::{File, Project};
use crate::queries::provider_of;
use crate::symbols;
use core_packages::Located;
use core_semantics::symbols::{
    FileSymbols, FunctionSymbol, Member, Name, NameKind, Table, TypeSymbol,
};
use std::collections::HashMap;
use std::path::Path;

/// What the index knows about a name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer<T> {
    Known(T),
    /// Proved not to exist.
    Absent,
    Unknown,
}

impl<T> Answer<T> {
    /// Whether the answer proves the name does not exist (the only answer a rule reports on).
    pub fn is_absent(&self) -> bool {
        matches!(self, Answer::Absent)
    }
}

/// Where a declaration is.
#[derive(Debug)]
pub struct Found<'db, T> {
    pub file: File,
    pub item: &'db T,
}

impl<T> Clone for Found<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Found<'_, T> {}

impl<T> PartialEq for Found<'_, T> {
    fn eq(&self, other: &Self) -> bool {
        self.file == other.file && std::ptr::eq(self.item, other.item)
    }
}

/// The names declared by a set of files, by language and name key, and whether the set can be
/// trusted to hold every name it declares.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Declared {
    types: HashMap<(usize, Box<str>), Vec<File>>,
    functions: HashMap<(usize, Box<str>), Vec<File>>,
    /// A file that could not be read as symbols (a syntax error, a failed parse), or declares
    /// names at run time: a name it may declare is not provably absent.
    blind: bool,
}

fn table_of(db: &dyn Db, file: File) -> Option<(usize, &Table)> {
    let provider = provider_of(db, file).ok()?;
    let lang = provider.symbols()?;
    let index = db
        .providers()
        .iter()
        .position(|p| std::ptr::eq(p.as_ref(), provider))?;
    Some((index, lang.table(provider.grammar())))
}

fn file_symbols(db: &dyn Db, file: File) -> Option<&FileSymbols> {
    symbols(db, file).as_ref().ok()?.as_ref()
}

fn declared(db: &dyn Db, files: impl Iterator<Item = File>) -> Declared {
    let mut d = Declared::default();
    for file in files {
        let Some((lang, table)) = table_of(db, file) else {
            continue;
        };
        let Some(s) = file_symbols(db, file) else {
            d.blind = true;
            continue;
        };
        d.blind |= !s.declaring_calls.is_empty();
        let facts = &table.facts;
        for t in &s.types {
            let key = (lang, facts.key(NameKind::Type, &t.name).into());
            d.types.entry(key).or_default().push(file);
        }
        for f in &s.functions {
            let key = (lang, facts.key(NameKind::Function, &f.name).into());
            d.functions.entry(key).or_default().push(file);
        }
    }
    d
}

/// What the project's own files declare.
#[salsa::tracked(returns(ref))]
fn project_declared(db: &dyn Db, project: Project) -> Declared {
    declared(db, project.files(db).iter().copied())
}

/// What the files the locator lists as autoloaded declare, read on first use.
#[salsa::tracked(returns(ref))]
fn autoload_declared(db: &dyn Db, project: Project) -> Declared {
    let _ = project.revision(db);
    let files = project
        .locator(db)
        .iter()
        .flat_map(|l| l.autoloaded())
        .filter_map(|p| db.dependency_file(&p))
        .collect::<Vec<_>>();
    declared(db, files.into_iter())
}

/// The index of one project, for the queries of one run or one editor request.
pub struct Index<'db> {
    db: &'db dyn Db,
    project: Project,
}

/// The depth of an inheritance chain the index follows before it says `Unknown`.
const MAX_CHAIN: usize = 32;

impl<'db> Index<'db> {
    pub fn new(db: &'db dyn Db, project: Project) -> Self {
        Index { db, project }
    }

    /// The type `name` (the qualified names a written name can stand for, tried in order) as
    /// seen from `from`.
    pub fn type_(&self, from: File, name: &Name) -> Answer<Found<'db, TypeSymbol>> {
        let Some((lang, table)) = table_of(self.db, from) else {
            return Answer::Unknown;
        };
        for candidate in name.iter() {
            let found = if table.sees_imports_only() {
                self.through_imports(from, table, candidate)
            } else {
                self.by_name(lang, table, candidate)
            };
            match found {
                Answer::Absent => {}
                answer => return answer,
            }
        }
        Answer::Absent
    }

    /// The function `name`: found in the project or an autoloaded file, else `Unknown` (the
    /// language's own functions are not known to the index).
    pub fn function(&self, from: File, name: &Name) -> Answer<Found<'db, FunctionSymbol>> {
        let Some((lang, table)) = table_of(self.db, from) else {
            return Answer::Unknown;
        };
        let sets = [
            project_declared(self.db, self.project),
            autoload_declared(self.db, self.project),
        ];
        for candidate in name.iter() {
            let key = (lang, table.facts.key(NameKind::Function, candidate).into());
            for set in sets {
                match set.functions.get(&key).map(Vec::as_slice) {
                    Some([file]) => {
                        let hit = file_symbols(self.db, *file).and_then(|s| {
                            s.functions.iter().find(|f| {
                                table.facts.key(NameKind::Function, &f.name)
                                    == table.facts.key(NameKind::Function, candidate)
                            })
                        });
                        return match hit {
                            Some(f) if !f.conditional => Answer::Known(Found {
                                file: *file,
                                item: f,
                            }),
                            _ => Answer::Unknown,
                        };
                    }
                    Some(_) => return Answer::Unknown,
                    None => {}
                }
            }
        }
        Answer::Unknown
    }

    /// The member `member` of the type `ty`, or of an ancestor or used trait.
    pub fn member(&self, from: File, ty: &Name, member: &str) -> Answer<Found<'db, Member>> {
        self.member_in(from, ty, member, 0)
    }

    fn member_in(
        &self,
        from: File,
        ty: &Name,
        member: &str,
        depth: usize,
    ) -> Answer<Found<'db, Member>> {
        let Some((_, table)) = table_of(self.db, from) else {
            return Answer::Unknown;
        };
        let found = match self.type_(from, ty) {
            Answer::Known(f) => f,
            Answer::Absent => return Answer::Absent,
            Answer::Unknown => return Answer::Unknown,
        };
        let t = found.item;
        if let Some(m) = t.member(&table.facts, member) {
            return Answer::Known(Found {
                file: found.file,
                item: m,
            });
        }
        if t.dynamic || t.incomplete || t.adapted || t.conditional || depth >= MAX_CHAIN {
            return Answer::Unknown;
        }
        let mut unknown = false;
        for parent in t.parents.iter().map(|p| &p.name).chain(&t.mixins) {
            match self.member_in(found.file, parent, member, depth + 1) {
                found @ Answer::Known(_) => return found,
                Answer::Unknown => unknown = true,
                Answer::Absent => {}
            }
        }
        if unknown {
            Answer::Unknown
        } else {
            Answer::Absent
        }
    }

    /// One qualified name, by the project's files and then the locator.
    fn by_name(
        &self,
        lang: usize,
        table: &Table,
        candidate: &str,
    ) -> Answer<Found<'db, TypeSymbol>> {
        let key = (lang, table.facts.key(NameKind::Type, candidate).into());
        let project = project_declared(self.db, self.project);
        if let Some(files) = project.types.get(&key) {
            return self.the_type(table, files, candidate);
        }
        let autoloaded = autoload_declared(self.db, self.project);
        if let Some(files) = autoloaded.types.get(&key) {
            return self.the_type(table, files, candidate);
        }
        let Some(locator) = self.project.locator(self.db) else {
            return Answer::Unknown;
        };
        match locator.type_files(candidate) {
            Located::Files(paths) => {
                let files: Vec<File> = paths
                    .iter()
                    .filter_map(|p| self.db.dependency_file(Path::new(p)))
                    .collect();
                let hits: Vec<File> = files
                    .iter()
                    .copied()
                    .filter(|f| {
                        file_symbols(self.db, *f).is_some_and(|s| {
                            s.types.iter().any(|t| {
                                table.facts.key(NameKind::Type, &t.name)
                                    == table.facts.key(NameKind::Type, candidate)
                            })
                        })
                    })
                    .collect();
                if hits.is_empty() {
                    Answer::Unknown
                } else {
                    self.the_type(table, &hits, candidate)
                }
            }
            Located::Missing
                if self.project.complete(self.db) && !project.blind && !autoloaded.blind =>
            {
                Answer::Absent
            }
            _ => Answer::Unknown,
        }
    }

    /// The declaration of `candidate` among `files`: one unconditional declaration is known,
    /// several or a conditional one are not.
    fn the_type(
        &self,
        table: &Table,
        files: &[File],
        candidate: &str,
    ) -> Answer<Found<'db, TypeSymbol>> {
        let want = table.facts.key(NameKind::Type, candidate);
        let mut hits = files.iter().flat_map(|&file| {
            file_symbols(self.db, file)
                .into_iter()
                .flat_map(|s| s.types.iter())
                .filter(|t| table.facts.key(NameKind::Type, &t.name) == want)
                .map(move |item| Found { file, item })
        });
        match (hits.next(), hits.next()) {
            (Some(found), None) if !found.item.conditional => Answer::Known(found),
            _ => Answer::Unknown,
        }
    }

    /// The files `from` sees: itself, what it imports and what those import publicly.
    fn through_imports(
        &self,
        from: File,
        table: &Table,
        candidate: &str,
    ) -> Answer<Found<'db, TypeSymbol>> {
        let Some(locator) = self.project.locator(self.db) else {
            return Answer::Unknown;
        };
        let mut seen = vec![from];
        let mut pending = vec![(from, true)];
        while let Some((file, direct)) = pending.pop() {
            let Some(s) = file_symbols(self.db, file) else {
                return Answer::Unknown;
            };
            for import in s.imports.iter().filter(|i| direct || i.public) {
                let path = match locator.import_file(&import.path) {
                    Located::Files(paths) => paths.into_iter().next(),
                    _ => None,
                };
                let Some(found) = path.and_then(|p| self.file_at(&p)) else {
                    return Answer::Unknown;
                };
                if !seen.contains(&found) {
                    seen.push(found);
                    pending.push((found, false));
                }
            }
        }
        match self.the_type(table, &seen, candidate) {
            Answer::Unknown if self.declared_anywhere(table, &seen, candidate) => Answer::Unknown,
            Answer::Unknown => Answer::Absent,
            known => known,
        }
    }

    fn declared_anywhere(&self, table: &Table, files: &[File], candidate: &str) -> bool {
        let want = table.facts.key(NameKind::Type, candidate);
        files.iter().any(|&f| {
            file_symbols(self.db, f).is_some_and(|s| {
                s.types
                    .iter()
                    .any(|t| table.facts.key(NameKind::Type, &t.name) == want)
            })
        })
    }

    /// The input of a file by path: a project file, else one read on demand.
    fn file_at(&self, path: &Path) -> Option<File> {
        self.project
            .files(self.db)
            .iter()
            .copied()
            .find(|f| f.path(self.db) == path)
            .or_else(|| self.db.dependency_file(path))
    }
}
