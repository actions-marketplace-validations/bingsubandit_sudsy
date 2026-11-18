//! The regex flavors a grammar's `regex` and `regex-call` rows name: what each dialect writes
//! the way the checks need to know.
//!
//! concept: regex-source-checks
//! what: Rules about the source of regular expressions in literals and call arguments, by flavor.
//! tools: eslint no-control-regex no-regex-spaces no-empty-character-class no-misleading-character-class, ruff regex checks
//! reads: grammar:regex*

/// The facts of one regex dialect.
pub struct Flavor {
    pub name: &'static str,
    /// `[]` is a class with no member (otherwise the `]` after `[` is a member).
    pub empty_class: bool,
    /// A class may hold classes (`[a[b]]`, `[a-z-[aeiou]]`).
    pub nested: bool,
    /// A class may hold `[:alpha:]`.
    pub posix: bool,
    /// The characters whose escape means something in every place: letters and digits that name
    /// a character, a set or an assertion, and the backslash and bracket.
    pub escapes: &'static str,
    /// The characters whose escape means something outside a class only (the metacharacters).
    pub outside: &'static str,
    /// Escape letters that stand for a set of characters (`\d`).
    pub sets: &'static str,
    /// Escape letters that start a property (`\p{L}`).
    pub properties: &'static str,
    /// Escape letters followed by a fixed number of hex digits (`\x41`, `A`).
    pub fixed: &'static [(char, usize)],
    /// Escape letters that may be followed by braced hex digits (`\u{41}`, `\x{41}`).
    pub braced: &'static str,
}

pub static FLAVORS: &[Flavor] = &[
    Flavor {
        name: "ecmascript",
        empty_class: true,
        nested: false,
        posix: false,
        escapes: "\\bcdDfnpPrsStvwWxu0123456789]",
        outside: "^/.$*+?[{}|()Bk",
        sets: "dDwWsS",
        properties: "pP",
        fixed: &[('x', 2), ('u', 4)],
        braced: "u",
    },
    Flavor {
        name: "pcre",
        empty_class: false,
        nested: false,
        posix: true,
        escapes: "\\abcdDeEfghHkKnNpPQrRsStuUvVwWxXzZAGB0123456789]",
        outside: "^.$*+?[{}|()",
        sets: "dDhHsSvVwWNRX",
        properties: "pP",
        fixed: &[('x', 2)],
        braced: "x",
    },
    Flavor {
        name: "python",
        empty_class: false,
        nested: false,
        posix: false,
        escapes: "\\abBdDfnrsStvwWxuUNAZ0123456789]",
        outside: "^.$*+?[{}|()",
        sets: "dDsSwW",
        properties: "",
        fixed: &[('x', 2), ('u', 4), ('U', 8)],
        braced: "",
    },
    Flavor {
        name: "java",
        empty_class: false,
        nested: true,
        posix: false,
        escapes: "\\abBcdDeEfhHnpPQrRsStuvVwWxXzZAG0123456789]",
        outside: "^.$*+?[{}|()",
        sets: "dDhHsSvVwWRX",
        properties: "pP",
        fixed: &[('x', 2), ('u', 4)],
        braced: "x",
    },
    Flavor {
        name: "dotnet",
        empty_class: false,
        nested: true,
        posix: false,
        escapes: "\\abBcdDefknpPrsStuvwWxAZzG0123456789]",
        outside: "^.$*+?[{}|()",
        sets: "dDsSwW",
        properties: "pP",
        fixed: &[('x', 2), ('u', 4)],
        braced: "",
    },
    Flavor {
        name: "onigmo",
        empty_class: false,
        nested: true,
        posix: true,
        escapes: "\\abBcdDefghHkKnpPrRsStuvwWxXzZAG0123456789]",
        outside: "^.$*+?[{}|()",
        sets: "dDhHsSwWR",
        properties: "pP",
        fixed: &[('x', 2), ('u', 4)],
        braced: "xu",
    },
];

/// The flavor a grammar row names.
pub fn named(name: &str) -> Option<&'static Flavor> {
    FLAVORS.iter().find(|f| f.name == name)
}

impl Flavor {
    /// Does the backslash escape nothing: the character is not one the dialect gives a meaning to
    /// at that place? Inside a class, `\^` first and `\-` between two members still mean
    /// something.
    pub fn useless(&self, e: &super::parse::Esc) -> bool {
        if e.in_class {
            if e.ch == '^' && e.after_open || e.ch == '-' && !e.after_open && !e.before_close {
                return false;
            }
            return !self.escapes.contains(e.ch);
        }
        !(self.escapes.contains(e.ch) || self.outside.contains(e.ch))
    }
}
