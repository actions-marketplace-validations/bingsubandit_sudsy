//! Markup elements whose content goes on lines of its own once the element has attributes, or has
//! an element among its children, also when it would fit on the line of its start tag. The
//! language names the elements that keep their content on the line (its inline ones) and says
//! whether the whitespace between the children is free.
//!
//! concept: attribute-content-lines
//! what: Whether an element's content goes on its own lines because of its attributes or its element children.
//! tools: eslint-plugin-vue singleline-html-element-content-newline, html-eslint element-newline
//! reads: option:content_newline_with_attributes, option:content_newline_with_elements

use core_config::ResolvedOptions;

#[derive(Clone, Copy)]
pub struct ContentLines {
    on: bool,
    elements: bool,
    keep: &'static [&'static str],
}

impl ContentLines {
    /// `keep`: the element names that keep their content on the line.
    pub fn of(options: &ResolvedOptions, keep: &'static [&'static str]) -> Self {
        ContentLines {
            on: options.bool(core_config::opt::CONTENT_NEWLINE_WITH_ATTRIBUTES),
            elements: options.bool(core_config::opt::CONTENT_NEWLINE_WITH_ELEMENTS),
            keep,
        }
    }

    /// The printer's `hug` (an interpolation that is the only child sits against the tags) and
    /// `force_break` (the content breaks onto lines) for an element `name` with `attributes`
    /// attributes: an element whose content gets lines of its own neither hugs nor fits on a
    /// line; every other element keeps the printer's values. `free_elements`: the element has an
    /// element child and the whitespace between its children is not significant.
    pub fn decide(
        &self,
        name: &str,
        attributes: usize,
        free_elements: bool,
        hug: bool,
        force_break: bool,
    ) -> (bool, bool) {
        let by_attributes = self.on && attributes > 0 && !self.keep.contains(&name);
        if by_attributes || (self.elements && free_elements) {
            (false, true)
        } else {
            (hug, force_break)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEEP: &[&str] = &["span", "pre"];

    fn lines(on: bool) -> ContentLines {
        ContentLines {
            on,
            elements: on,
            keep: KEEP,
        }
    }

    #[test]
    fn option_off_changes_nothing() {
        assert_eq!(
            lines(false).decide("p", 1, false, true, false),
            (true, false)
        );
    }

    #[test]
    fn an_element_with_attributes_breaks_and_does_not_hug() {
        assert_eq!(
            lines(true).decide("p", 2, false, true, false),
            (false, true)
        );
    }

    #[test]
    fn free_element_children_break() {
        assert_eq!(
            lines(true).decide("li", 0, true, true, false),
            (false, true)
        );
        assert_eq!(
            lines(false).decide("li", 0, true, true, false),
            (true, false)
        );
    }

    #[test]
    fn kept_elements_and_elements_without_attributes_are_left_alone() {
        assert_eq!(
            lines(true).decide("span", 1, false, true, false),
            (true, false)
        );
        assert_eq!(
            lines(true).decide("pre", 1, false, false, true),
            (false, true)
        );
        assert_eq!(
            lines(true).decide("p", 0, false, true, false),
            (true, false)
        );
    }
}
