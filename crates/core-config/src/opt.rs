//! Option ids: every option name has a numeric id (`OptId`), the index of the name in the sorted
//! list `cargo xtask codegen` writes from the options of all languages and from
//! `crates/core-config/options.txt`. Resolved options are a vector indexed by id, so reading one
//! is an array access, never a lookup by name. A name is turned into an id once, where data names
//! an option (a row, a config key); code names it by the constant of this module.
//!
//! concept: none (infrastructure: option ids)

/// The id of an option name; see the module docs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OptId(pub(crate) u16);

impl OptId {
    pub fn index(self) -> usize {
        self.0 as usize
    }

    pub fn name(self) -> &'static str {
        NAMES[self.index()]
    }
}

/// What names an option where a schema is built: an id, or a name that is looked up.
pub trait IntoOpt {
    fn id(self) -> OptId;
}

impl IntoOpt for OptId {
    fn id(self) -> OptId {
        self
    }
}

impl IntoOpt for &str {
    fn id(self) -> OptId {
        must(self)
    }
}

/// The id of a name; `None` when no language and no shared list declares it.
pub fn id(name: &str) -> Option<OptId> {
    NAMES.binary_search(&name).ok().map(|i| OptId(i as u16))
}

/// The id of a name that data (a row, a kind-set key) is known to carry; panics on an unknown
/// name, so a typo fails where the data is read, not silently at every use.
pub fn must(name: &str) -> OptId {
    id(name).unwrap_or_else(|| {
        panic!("unknown option `{name}`: not in a lang.yaml `options` and not in core-config/options.txt")
    })
}

include!("opt_ids.generated.rs");
