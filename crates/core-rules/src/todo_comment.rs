//! The family `todo-comment`: a line of a comment (a token tagged `#comment`) that starts with
//! the param `word` (`todo`) in any case, as a whole word, and not with the param `required`
//! (`TODO:`: the word in its case and the punctuation that follows it). The first line is read
//! after the comment's opening (`//`, `/*`), the next lines after their blanks; a line that
//! starts with an asterisk is a doc comment line and is left alone, so are doc comments.
//!
//! concept: todo-comment
//! what: A to-do marker in a comment that is not written in the one form the style asks for.
//! tools: Checkstyle TodoComment
//! rules: todo-comment

use crate::{Family, FileContext, Params, Rule, RuleContext, RuleMeta};
use core_syntax::TextRange;

pub const FAMILY: Family = Family {
    name: "todo-comment",
    build,
};

struct TodoComment {
    meta: &'static RuleMeta,
    message: String,
    word: String,
    required: String,
}

fn build(meta: &'static RuleMeta, mut p: Params) -> Result<Box<dyn Rule>, String> {
    let message = p.text("message")?;
    let word = p.text("word")?.to_lowercase();
    let required = p.text("required")?;
    p.done()?;
    Ok(Box::new(TodoComment {
        meta,
        message,
        word,
        required,
    }))
}

impl TodoComment {
    /// Is the text a marker the style does not allow?
    fn wrong(&self, line: &str) -> bool {
        let n = self.word.len();
        let Some(head) = line.get(..n) else {
            return false;
        };
        head.to_lowercase() == self.word
            && !line[n..].starts_with(|c: char| c.is_alphanumeric() || c == '_')
            && !line.starts_with(&self.required)
    }
}

impl Rule for TodoComment {
    fn meta(&self) -> &'static RuleMeta {
        self.meta
    }

    fn check_file(&self, file: &FileContext<'_>, ctx: &mut RuleContext<'_>) {
        if !file.text.to_ascii_lowercase().contains(&self.word) {
            return;
        }
        let g = file.grammar;
        for t in file
            .root
            .descendants_with_tokens()
            .filter_map(|e| e.into_token())
            .filter(|t| g.has_tag(t.kind(), "comment"))
        {
            let text = t.text();
            if text.starts_with("/**") && text.len() > 4 {
                continue;
            }
            let at = usize::from(t.text_range().start());
            let mut offset = 0;
            for (i, line) in text.split('\n').enumerate() {
                let body = if i == 0 {
                    line.strip_prefix("//")
                        .or_else(|| line.strip_prefix("/*"))
                        .unwrap_or(line)
                } else {
                    line
                };
                let skipped = line.len() - body.len();
                let lead = body.len() - body.trim_start_matches([' ', '\t']).len();
                if self.wrong(&body[lead..]) {
                    let start = at + offset + skipped + lead;
                    ctx.report(
                        TextRange::new(
                            (start as u32).into(),
                            ((start + self.word.len()) as u32).into(),
                        ),
                        &self.message,
                    );
                }
                offset += line.len() + 1;
            }
        }
    }
}
