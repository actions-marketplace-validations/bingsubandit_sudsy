//! Statement shapes of postgres that the generic walker does not read from position alone.
//!
//! concept: recursive-descent-parser

use super::P;
use super::lexer::T;
use crate::generated::kind as k;

impl P<'_> {
    /// `AGGREGATE name (signature)`: the name is plain (no function spacing); in a `CREATE` the
    /// arguments are types, elsewhere they are words as written. The bracket keeps its blank in
    /// an `ALTER` and where it holds an `ORDER BY`.
    pub fn aggregate_signature(&mut self, create: bool, alter: bool) {
        self.generic_name(false);
        if !self.is_t(0, T::LParen) {
            return;
        }
        let m = self.mark();
        let mut ordered = false;
        self.bracketed(|p| {
            while let Some(t) = p.t(0) {
                ordered |= p.at("order");
                match t {
                    T::RParen => break,
                    T::Word if p.at_any(&["in", "out", "inout", "variadic", "order", "by"]) => {
                        p.bump_as(k::WORD)
                    }
                    T::Word if create => {
                        if !p.type_name() {
                            p.bump();
                        }
                    }
                    T::Word => p.bump_as(k::WORD),
                    _ => p.bump(),
                }
            }
        });
        if !(alter || ordered) {
            self.close(k::FUNCTION_CONTENTS, m);
        }
    }
}
