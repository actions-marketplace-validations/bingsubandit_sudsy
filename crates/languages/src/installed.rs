//! Which languages are installed. Every build links all languages in and `Checker` finds nothing
//! missing; pack mode (feature `pack-mode`) adds the installed set (`pack.rs`).

#[cfg(feature = "pack-mode")]
mod pack;
#[cfg(feature = "pack-mode")]
pub use pack::*;

#[cfg(not(feature = "pack-mode"))]
pub use all::Checker;

#[cfg(not(feature = "pack-mode"))]
mod all {
    use provider_api::LanguageProvider;
    use std::marker::PhantomData;
    use std::path::Path;

    /// Every language is built in: no file is ever missing.
    pub struct Checker<'a>(PhantomData<&'a ()>);

    impl<'a> Checker<'a> {
        pub fn new(_: &'a [Box<dyn LanguageProvider>]) -> Self {
            Checker(PhantomData)
        }

        pub fn missing(&mut self, _: &Path) -> Option<(&'a str, String)> {
            None
        }
    }
}
