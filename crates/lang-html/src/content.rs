//! The content model of HTML (what each element may hold): the four shared checks over
//! `data/model.txt`.
//!
//! concept: content-model

use core_rules::content_model::Model;
use core_rules::content_rule;
use std::sync::LazyLock;

pub(crate) fn model() -> &'static Model {
    static MODEL: LazyLock<Model> =
        LazyLock::new(|| Model::parse(include_str!("../data/model.txt")));
    &MODEL
}

content_rule! {
    pub PERMITTED_CONTENT {
        id: "element-permitted-content",
        category: Correctness,
        description: "An element that its parent does not permit as content (`<li>` outside a list, \
                  `<div>` in a `<table>`, a deprecated element in flow content), or one that an ancestor \
                  forbids as a descendant (`<a>` in `<a>`, a block in `<p>`'s `<button>`).",
        example: "<table><div></div></table>",
        check: Content,
        model: model,
    }
}

content_rule! {
    pub PERMITTED_ORDER {
        id: "element-permitted-order",
        category: Correctness,
        description: "Children out of the order the parent requires (`<tbody>` before `<tfoot>`, `<head>` \
                  before `<body>`, `<summary>` first in `<details>`).",
        example: "<table><tbody></tbody><thead></thead></table>",
        check: Order,
        model: model,
    }
}

content_rule! {
    pub PERMITTED_OCCURRENCES {
        id: "element-permitted-occurrences",
        category: Correctness,
        description: "An element the parent allows at most once (`<head>`, `<body>`, `<title>`, `<caption>`) \
                  that appears again. A `<script>` or `<style>` after `</body>` opens an implied `<head>`.",
        example: "<html><head></head><head></head></html>",
        check: Occurrences,
        model: model,
    }
}

content_rule! {
    pub REQUIRED_CONTENT {
        id: "element-required-content",
        category: Correctness,
        description: "An element that lacks a child it requires (`<title>` in `<head>`, `<summary>` in \
                  `<details>`, `<head>` and `<body>` in `<html>`).",
        example: "<head></head>",
        check: Required,
        model: model,
    }
}
