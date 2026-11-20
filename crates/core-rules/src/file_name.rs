//! The `file-name` family: the name of a file follows a convention an option sets. Params:
//! - `option`: the name of the `regex` option (declared in the file) the file's base name must
//!   match; empty: not checked;
//! - `files` (optional): a regex of the path (with `/`) of the files that are checked, the
//!   others are left alone;
//! - `until` (optional): the base name ends at the first of this character (`.`: the name of
//!   `Job.common.kt` is `Job`); by default it is the name without its last extension;
//! - `message`: with `{name}` (the base name) and `{convention}` (the option's value).
//!
//! The diagnostic is an empty span at the start of the file.
//!
//! concept: file-name
//! what: The base name of a file matches a regex option.
//! rules: file-name
//! reads: family:file-name

use crate::{Family, FileContext, Params, RegexCache, Rule, RuleContext, RuleMeta};
use core_syntax::TextRange;

pub const FAMILY: Family = Family {
    name: "file-name",
    build,
};

struct FileName {
    meta: &'static RuleMeta,
    message: String,
    option: String,
    files: Option<String>,
    until: Option<char>,
    cache: RegexCache,
}

fn build(meta: &'static RuleMeta, mut p: Params) -> Result<Box<dyn Rule>, String> {
    let message = p.text("message")?;
    let option = p.text("option")?;
    let files = p.optional("files");
    let until = p.optional("until").and_then(|u| u.chars().next());
    p.done()?;
    Ok(Box::new(FileName {
        meta,
        message,
        option,
        files,
        until,
        cache: RegexCache::default(),
    }))
}

impl Rule for FileName {
    fn meta(&self) -> &'static RuleMeta {
        self.meta
    }

    fn check_file(&self, file: &FileContext<'_>, ctx: &mut RuleContext<'_>) {
        let convention = ctx.options.string(&self.option);
        if convention.is_empty() {
            return;
        }
        let path = file.path.to_string_lossy().replace('\\', "/");
        if self
            .files
            .as_deref()
            .is_some_and(|f| !self.cache.is_match(f, &path))
        {
            return;
        }
        let file_name = path.rsplit('/').next().unwrap_or(&path);
        let name = match self.until {
            Some(c) => file_name.split(c).next().unwrap_or(file_name),
            None => file_name.rsplit_once('.').map_or(file_name, |(n, _)| n),
        };
        if !self.cache.is_match(convention, name) {
            let message = self
                .message
                .replace("{name}", name)
                .replace("{convention}", convention);
            ctx.report(TextRange::empty(0.into()), message);
        }
    }
}
