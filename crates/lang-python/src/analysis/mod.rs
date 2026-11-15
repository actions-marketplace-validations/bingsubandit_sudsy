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
use core_syntax::SyntaxNode;
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

/// The checks the `check` family accepts.
pub const CHECKS: &[&str] = &[
    "ambiguous-class-name",
    "ambiguous-function-name",
    "ambiguous-variable-name",
    "bare-except",
    "break-outside-loop",
    "continue-outside-loop",
    "default-except-not-last",
    "f-string-missing-placeholders",
    "forward-annotation-syntax-error",
    "future-feature-not-defined",
    "import-shadowed-by-loop-var",
    "invalid-print-syntax",
    "invalid-syntax",
    "is-literal",
    "lambda-assignment",
    "late-future-import",
    "module-import-not-at-top-of-file",
    "multi-value-repeated-key-literal",
    "multi-value-repeated-key-variable",
    "multiple-starred-expressions",
    "multiple-statements-on-one-line-colon",
    "multiple-statements-on-one-line-semicolon",
    "percent-format-expected-mapping",
    "percent-format-expected-sequence",
    "percent-format-extra-named-arguments",
    "percent-format-invalid-format",
    "percent-format-missing-argument",
    "percent-format-mixed-positional-and-named",
    "percent-format-positional-count-mismatch",
    "percent-format-star-requires-sequence",
    "percent-format-unsupported-format-character",
    "redefined-while-unused",
    "return-outside-function",
    "string-dot-format-extra-named-arguments",
    "string-dot-format-extra-positional-arguments",
    "string-dot-format-invalid-format",
    "string-dot-format-missing-arguments",
    "string-dot-format-mixing-automatic",
    "type-comparison",
    "undefined-export",
    "undefined-local",
    "undefined-local-with-import-star",
    "undefined-local-with-import-star-usage",
    "undefined-local-with-nested-import-star-usage",
    "undefined-name",
    "unused-annotation",
    "unused-import",
    "unused-variable",
    "useless-semicolon",
    "yield-outside-function",
];

struct Kept {
    src: String,
    parsed: Parsed<Mod>,
    target: PythonVersion,
}

thread_local! {
    static PARSED: RefCell<Option<Kept>> = const { RefCell::new(None) };
    static HITS: RefCell<Option<(SyntaxNode, Rc<Vec<Hit>>)>> = const { RefCell::new(None) };
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
        let mut cache = cache.borrow_mut();
        if let Some((root, hits)) = &*cache
            && root == file.root
        {
            return hits.clone();
        }
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
        let hits = Rc::new(analyze(file, &parsed, target));
        *cache = Some((file.root.clone(), hits.clone()));
        hits
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
    let mut hits = ck.m.hits;
    hits.sort_by_key(|h| (h.start, h.end));
    hits
}
