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
        description: "Reports a link (`<a href>`) with no text, no image with an `alt` and no \
              `aria-label` (WCAG H30). A link hidden from assistive technology is not reported.",
        why: "A screen reader announces the link as \"link\" with no purpose, and a list of the \
              links of a page shows an empty entry. Sighted users get a link they cannot see or \
              have no name for.\n\
              To fix, put text in the link, or an image with an `alt`, or add `aria-label`.",
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
        description: "Reports a `<form>` that has no submit button inside it and none that names it with \
              `form=` (WCAG H32).",
        why: "Without a submit button a form can only be sent by pressing Enter in a field, \
              which not every user knows, and which does nothing in a form with several fields \
              in some browsers.\n\
              To fix, add a `<button type=\"submit\">` (or `<input type=\"submit\">`) to the form.",
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
        description: "Reports a `<th>` with no `scope` (`row`, `col`, `rowgroup` or `colgroup`) in a \
              table that is not a simple one. A table with a single row or a single column of \
              headers is simple (WCAG H63).",
        why: "In a table with headers on both axes a screen reader cannot tell whether a header \
              belongs to the cells beside it or below it, so a cell is read with the wrong \
              header.\n\
              To fix, add the `scope` that says which cells the header describes.",
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
        description: "Reports an element that can take focus (a link, a control, an element with \
              `tabindex` of 0 or more) inside an element with `aria-hidden=\"true\"`.",
        why: "Keyboard users can still reach the element, but assistive technology does not \
              announce it, so focus goes to something that is not there for the screen reader \
              user.\n\
              To fix, remove `aria-hidden`, or make the content not focusable (`tabindex=\"-1\"`, \
              `disabled`, `inert`).",
        example: "<div aria-hidden=\"true\"><button>x</button></div>",
        check: HiddenFocusable,
        model: model,
        words: &WORDS,
    }
}
