//! Markup elements whose content goes on lines of its own once the element has attributes, also
//! when it would fit on the line of its start tag. The language names the elements that keep
//! their content on the line (its inline ones).
//!
//! concept: attribute-content-lines
//! what: Whether an element's content goes on its own lines because the element has attributes.
//! tools: eslint-plugin-vue singleline-html-element-content-newline
//! reads: option:content_newline_with_attributes

use core_config::ResolvedOptions;

#[derive(Clone, Copy)]
pub struct ContentLines {
    on: bool,
    keep: &'static [&'static str],
}

impl ContentLines {
    /// `keep`: the element names that keep their content on the line.
    pub fn of(options: &ResolvedOptions, keep: &'static [&'static str]) -> Self {
        ContentLines {
            on: options.bool(core_config::opt::CONTENT_NEWLINE_WITH_ATTRIBUTES),
            keep,
        }
    }

    /// The printer's `hug` (an interpolation that is the only child sits against the tags) and
    /// `force_break` (the content breaks onto lines) for an element `name` with `attributes`
    /// attributes: an element whose content gets lines of its own neither hugs nor fits on a
    /// line; every other element keeps the printer's values.
    pub fn decide(
        &self,
        name: &str,
        attributes: usize,
        hug: bool,
        force_break: bool,
    ) -> (bool, bool) {
        if self.on && attributes > 0 && !self.keep.contains(&name) {
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
        ContentLines { on, keep: KEEP }
    }

    #[test]
    fn option_off_changes_nothing() {
        assert_eq!(lines(false).decide("p", 1, true, false), (true, false));
    }

    #[test]
    fn an_element_with_attributes_breaks_and_does_not_hug() {
        assert_eq!(lines(true).decide("p", 2, true, false), (false, true));
    }

    #[test]
    fn kept_elements_and_elements_without_attributes_are_left_alone() {
        assert_eq!(lines(true).decide("span", 1, true, false), (true, false));
        assert_eq!(lines(true).decide("pre", 1, false, true), (false, true));
        assert_eq!(lines(true).decide("p", 0, true, false), (true, false));
    }
}
