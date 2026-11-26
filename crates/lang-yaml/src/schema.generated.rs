// Generated from lang.yaml by `cargo xtask codegen`.
#[allow(dead_code)]
pub const SEQUENCE_INDENT: core_config::OptId = core_config::opt::SEQUENCE_INDENT;
#[allow(dead_code)]
pub const QUOTE_STYLE: core_config::OptId = core_config::opt::QUOTE_STYLE;
#[allow(dead_code)]
pub const DOCUMENT_START: core_config::OptId = core_config::opt::DOCUMENT_START;
#[allow(dead_code)]
pub const DOCUMENT_END: core_config::OptId = core_config::opt::DOCUMENT_END;
#[allow(dead_code)]
pub const BLANK_LINES_AFTER_KEY: core_config::OptId = core_config::opt::BLANK_LINES_AFTER_KEY;
#[allow(dead_code)]
pub const COMMENT_SPACING: core_config::OptId = core_config::opt::COMMENT_SPACING;
#[allow(dead_code)]
pub const COMMENT_INDENT: core_config::OptId = core_config::opt::COMMENT_INDENT;
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SequenceIndent { Preserve, Indented, Flush }
#[allow(dead_code)]
pub fn sequence_indent(o: &core_config::ResolvedOptions) -> SequenceIndent {
    match o.text(SEQUENCE_INDENT) {
        "indented" => SequenceIndent::Indented,
        "flush" => SequenceIndent::Flush,
        _ => SequenceIndent::Preserve,
    }
}
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuoteStyle { Preserve, Single, Double, PreferDouble }
#[allow(dead_code)]
pub fn quote_style(o: &core_config::ResolvedOptions) -> QuoteStyle {
    match o.text(QUOTE_STYLE) {
        "single" => QuoteStyle::Single,
        "double" => QuoteStyle::Double,
        "prefer_double" => QuoteStyle::PreferDouble,
        _ => QuoteStyle::Preserve,
    }
}
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocumentStart { Preserve, Always, Never }
#[allow(dead_code)]
pub fn document_start(o: &core_config::ResolvedOptions) -> DocumentStart {
    match o.text(DOCUMENT_START) {
        "always" => DocumentStart::Always,
        "never" => DocumentStart::Never,
        _ => DocumentStart::Preserve,
    }
}
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocumentEnd { Preserve, Always, Never }
#[allow(dead_code)]
pub fn document_end(o: &core_config::ResolvedOptions) -> DocumentEnd {
    match o.text(DOCUMENT_END) {
        "always" => DocumentEnd::Always,
        "never" => DocumentEnd::Never,
        _ => DocumentEnd::Preserve,
    }
}
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlankLinesAfterKey { Preserve, Remove }
#[allow(dead_code)]
pub fn blank_lines_after_key(o: &core_config::ResolvedOptions) -> BlankLinesAfterKey {
    match o.text(BLANK_LINES_AFTER_KEY) {
        "remove" => BlankLinesAfterKey::Remove,
        _ => BlankLinesAfterKey::Preserve,
    }
}
#[allow(dead_code)]
pub fn comment_spacing(o: &core_config::ResolvedOptions) -> usize {
    o.uint(COMMENT_SPACING, 1)
}
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommentIndent { Preserve, Content }
#[allow(dead_code)]
pub fn comment_indent(o: &core_config::ResolvedOptions) -> CommentIndent {
    match o.text(COMMENT_INDENT) {
        "content" => CommentIndent::Content,
        _ => CommentIndent::Preserve,
    }
}

#[rustfmt::skip]
pub fn option_schema() -> core_config::OptionSchema {
    core_config::OptionSchema::lazy(|| {
        core_config::OptionSchema::new(vec![
        core_config::shared::line_width(80),
        core_config::shared::indent_size(2),
        core_config::shared::final_newline(true),
        shared::end_of_line(),
        core_config::shared::bracket_spacing(true),
        core_config::shared::array_bracket_spacing(false),
        core_config::OptionDef::choice(core_config::opt::SEQUENCE_INDENT, &["preserve", "indented", "flush"], "preserve", "A block sequence that is the value of a mapping key: `indented` puts the dashes one level in, `flush` at the key's column, `preserve` keeps each sequence as written."),
        core_config::OptionDef::choice(core_config::opt::QUOTE_STYLE, &["preserve", "single", "double", "prefer_double"], "preserve", "Quote style of quoted scalars. With `single` and `double` a scalar keeps its quotes when the other style would change its value or need more escapes; `prefer_double` uses double quotes, except single quotes for text that contains a `\"` (unless it has other escapes, which keep it as written)."),
        core_config::OptionDef::choice(core_config::opt::DOCUMENT_START, &["preserve", "always", "never"], "preserve", "The `---` marker: `always` starts every document with it, `never` leaves it out of the first document when that is possible, `preserve` keeps what is written."),
        core_config::OptionDef::choice(core_config::opt::DOCUMENT_END, &["preserve", "always", "never"], "preserve", "The `...` marker: `always` ends every document with it, `never` leaves it out unless the next document has a directive or no `---`, or the document is empty, `preserve` keeps what is written."),
        core_config::shared::max_blank_lines(1, "Largest number of blank lines kept between entries, comments and documents."),
        core_config::OptionDef::choice(core_config::opt::BLANK_LINES_AFTER_KEY, &["preserve", "remove"], "preserve", "Blank lines between a key and its block value on the next line: `preserve` keeps \n             them (up to `max_blank_lines`); `remove` drops them."),
        core_config::OptionDef::int(core_config::opt::COMMENT_SPACING, 1, (1, 10), "Spaces between a value and a comment after it on the same line."),
        core_config::OptionDef::choice(core_config::opt::COMMENT_INDENT, &["preserve", "content"], "preserve", "A comment on its own line after the end of a block collection: `preserve` keeps it with the outermost collection it is indented as deep as, `content` with the innermost one when it is deeper than the next token (so it is indented like the line before it or like the next token)."),
        ])
        .with_presets(vec![
        core_config::Preset::new("yaml/prettier-compat", include_str!("../presets/prettier-compat.yaml")),
        core_config::Preset::new("yaml/yamllint", include_str!("../presets/yamllint.yaml")),
        core_config::Preset::new("yaml/ansible-lint", include_str!("../presets/ansible-lint.yaml")),
        ])
    })
}
