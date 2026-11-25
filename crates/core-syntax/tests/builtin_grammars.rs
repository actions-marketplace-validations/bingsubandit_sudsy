use core_syntax::{LanguageId, grammars};

#[test]
fn builtin_grammars_load() {
    assert_eq!(grammars::json().language_id, LanguageId::JSON);
    assert_eq!(grammars::csharp().language_id, LanguageId::CSHARP);
    assert_eq!(grammars::yaml().language_id, LanguageId::YAML);
}

/// The mask codegen writes as `kind::TRIVIA_MASK` is `Grammar::is_trivia` of every token.
#[test]
fn trivia_mask_is_the_trivia_tokens() {
    for id in 0..=u8::MAX {
        let Some(g) = grammars::of(LanguageId::new(id)) else {
            continue;
        };
        if g.tokens()
            .iter()
            .any(|d| g.is_trivia(d.kind) && d.kind.index() >= 64)
        {
            continue; // no `TRIVIA_MASK` (C#)
        }
        for d in g.tokens() {
            assert_eq!(
                core_syntax::kind::in_mask(g.trivia_mask(), d.kind),
                g.is_trivia(d.kind),
                "{} {}",
                g.language,
                d.name
            );
        }
    }
}
