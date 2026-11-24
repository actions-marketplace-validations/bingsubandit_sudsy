//! The template language of `dialect: template`: the delimiters and block words of Go templates
//! (Hugo), read by the shared template-action code.
//!
//! concept: none (data: template action words)

use core_syntax::template_actions::Actions;

pub const ACTIONS: Actions = Actions {
    open: "{{",
    close: "}}",
    trim: '-',
    openers: &["if", "range", "with", "define", "block"],
    continuers: &["else"],
    closers: &["end"],
};
