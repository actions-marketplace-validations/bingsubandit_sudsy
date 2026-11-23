//! The accessibility checks over the element tree: the words of HTML for the shared checks.
//!
//! concept: markup-checks

use crate::content::model;
use core_rules::markup_checks::Words;
use core_rules::markup_rule;

static WORDS: Words = Words {
    link: "a",
    target: "href",
    image: "img",
    alt: "alt",
    label: "aria-label",
    hidden: "hidden",
    aria_hidden: "aria-hidden",
    inert: "inert",
    role: "role",
    presentation: &["presentation", "none"],
    tabindex: "tabindex",
    disabled: "disabled",
    style: "style",
    table: "table",
    header: "th",
    row: "tr",
    row_groups: &["thead", "tbody"],
    head_group: "thead",
    body_group: "tbody",
    scope: "scope",
    scopes: &["row", "col", "rowgroup", "colgroup"],
    headers: "headers",
    id: "id",
    form_attr: "form",
    form_category: "form",
    focusable_category: "focusable",
    submit_category: "submit",
    dynamic: "{{",
};

markup_rule! {
    pub LINK_TEXT {
        id: "link-text",
        category: Correctness,
        description: "A link (`<a href>`) with no text, no image with alt text and no `aria-label`: \
                  its purpose is not told to assistive technology (WCAG H30). Links hidden from it are left alone.",
        example: "<a href=\"/x\"><i class=\"icon\"></i></a>",
        check: LinkText,
        model: model,
        words: &WORDS,
    }
}

markup_rule! {
    pub FORM_SUBMIT {
        id: "form-submit",
        category: Correctness,
        description: "A `<form>` with no submit button inside it and none that names it with `form=` (WCAG H32).",
        example: "<form><input name=\"q\"></form>",
        check: FormSubmit,
        model: model,
        words: &WORDS,
    }
}

markup_rule! {
    pub HEADER_SCOPE {
        id: "header-scope",
        category: Correctness,
        description: "A `<th>` without `scope` (`row`, `col`, `rowgroup`, `colgroup`) in a table that is \
                  not a simple one (one row or one column of headers) (WCAG H63).",
        example: "<table><tr><th></th><th>b</th></tr><tr><th>c</th><td></td></tr></table>",
        check: HeaderScope,
        model: model,
        words: &WORDS,
    }
}

markup_rule! {
    pub HIDDEN_FOCUSABLE {
        id: "hidden-focusable",
        category: Correctness,
        description: "An element that can take focus (a link, a control, `tabindex` of 0 or more) inside \
                  `aria-hidden=\"true\"`: keyboard users reach it, assistive technology does not.",
        example: "<div aria-hidden=\"true\"><button>x</button></div>",
        check: HiddenFocusable,
        model: model,
        words: &WORDS,
    }
}
