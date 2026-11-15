//! What the rules that read names and bindings need: ruff's checks of its default set that look
//! at more than one node (unused and undefined names, redefinitions, scope errors, `__all__`,
//! `%` and `.format` strings, dict keys, `E4`/`E7` layout of statements). One walk over ruff's AST
//! computes every finding; the rules of the `check` family each report their own.
//!
//! The AST is the one the provider's parse built the tree from (`keep`); a file whose parse was
//! not kept (another thread, an editor request) is parsed again. Ranges are file offsets, the
//! same in the AST and the lossless tree.

mod checker;
mod dot_format;
mod finish;
mod formats;
mod model;
mod names;
mod semantic;
mod syntax;

use core_rules::FileContext;
use core_rules::checks::FileCache;
use ruff_python_ast::{Mod, PythonVersion};
use ruff_python_parser::{Mode, ParseOptions, Parsed, parse_unchecked};
use std::cell::RefCell;
use std::rc::Rc;

/// A finding of one check: its range, and the name the rule's message mentions.
pub struct Hit {
    pub check: &'static str,
    pub start: u32,
    pub end: u32,
    pub name: String,
}

struct Kept {
    src: String,
    parsed: Parsed<Mod>,
    target: PythonVersion,
}

thread_local! {
    static PARSED: RefCell<Option<Kept>> = const { RefCell::new(None) };
    static HITS: FileCache<Vec<Hit>> = const { FileCache::new() };
}

/// Keeps the parse of a file without errors for the lint that follows on this thread.
pub fn keep(src: &str, parsed: Parsed<Mod>, target: PythonVersion) {
    PARSED.with(|p| {
        *p.borrow_mut() = Some(Kept {
            src: src.to_owned(),
            parsed,
            target,
        });
    });
}

/// The findings of every check on the file, computed once and shared by the rules that run on it.
pub fn hits(file: &FileContext<'_>) -> Rc<Vec<Hit>> {
    HITS.with(|cache| {
        cache.get(file, || {
            let kept = PARSED
                .with(|p| p.borrow_mut().take())
                .filter(|k| k.src == file.text);
            let (parsed, target) = match kept {
                Some(k) => (k.parsed, k.target),
                None => {
                    let target = PythonVersion::latest();
                    let options = ParseOptions::from(Mode::Module).with_target_version(target);
                    (parse_unchecked(file.text, options), target)
                }
            };
            analyze(file, &parsed, target)
        })
    })
}

fn analyze(file: &FileContext<'_>, parsed: &Parsed<Mod>, target: PythonVersion) -> Vec<Hit> {
    let Mod::Module(module) = parsed.syntax() else {
        return Vec::new();
    };
    if !parsed.errors().is_empty() || !parsed.unsupported_syntax_errors().is_empty() {
        return Vec::new();
    }
    let mut ck = checker::Checker::new(file.text, file.path, target);
    ck.run(&module.body);
    syntax::semicolons(&mut ck.m, parsed.tokens());
    syntax::invalid_escapes(&mut ck.m, file.text, parsed.tokens());
    let mut hits = ck.m.hits;
    hits.sort_by_key(|h| (h.start, h.end));
    hits
}
