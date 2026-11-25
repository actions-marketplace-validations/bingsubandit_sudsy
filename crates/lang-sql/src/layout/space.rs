//! Spacing between blocks on one line (sqlfluff's LT01, LT06): the `[space]` rows of
//! `format.rules` say for each pair of neighbouring tokens whether one blank, none or the source's
//! stands between them, the `[joined]` rows whether a line break between them goes. The rows are
//! read once per file; a round of the layout only applies them to the current gaps.
//!
//! concept: token-spacing-rows

use super::seq::{Seq, is_comment};
use crate::generated::kind as k;

/// What the rows say about the gap before a block.
#[derive(Clone, Copy, Default)]
pub struct Gap {
    /// One blank (`Some(true)`), none (`Some(false)`) or as written (`None`).
    pub want: Option<bool>,
    /// A line break in the gap goes.
    pub join: bool,
}

/// The gap before each block (`pairs` as `SpaceRules::gaps` returns them); the first has none.
pub fn gaps(pairs: &[(Option<bool>, bool)]) -> Vec<Gap> {
    pairs
        .iter()
        .map(|&(want, join)| Gap { want, join })
        .collect()
}

const FUSION: core_format::spacing::Fusion = core_format::spacing::Fusion {
    operators: "+-*/<>=~!@#%^&|`?",
    sigils: "@#",
    line_comment: ['-', '-'],
};

/// Sets the gap of every point that sits inside a line; returns whether anything changed.
pub fn respace(seq: &mut Seq) -> bool {
    (1..seq.blocks.len()).fold(false, |changed, i| respace_point(seq, i, false) | changed)
}

/// Sets the gap before block `i`; `strip` removes a line break whatever the rows say.
pub fn respace_point(seq: &mut Seq, i: usize, strip: bool) -> bool {
    let Gap { want, join } = seq.gaps[i];
    // A `:` that starts a line is a sqlcmd command (`:r file`): the blank after it stays.
    let want = if seq.blocks[i - 1].kind == k::COLON && (i == 1 || seq.points[i - 1].newlines > 0) {
        None
    } else {
        want
    };
    let comment = is_comment(seq.blocks[i - 1].kind) || is_comment(seq.blocks[i].kind);
    let touch = want == Some(false);
    let any = want.is_none();
    let fuse = (touch || any)
        && core_format::spacing::fuses(
            &seq.blocks[i - 1].text,
            &seq.blocks[i].text,
            seq.blocks[i - 1].kind == k::SIGN,
            &FUSION,
        );
    let strip = (strip || join) && !comment && !fuse;
    // The characters of one operator (`+=`, `||`) the parser lexed apart stay together.
    let op = |j: usize| {
        let b = &seq.blocks[j];
        (b.kind == k::OPERATOR || b.kind == k::BINARY_OPERATOR)
            && !b.text.chars().any(|c| c.is_alphanumeric() || c == '_')
    };
    let joined = op(i - 1) && op(i);
    let p = &mut seq.points[i];
    if joined && p.newlines == 0 && p.ws.is_empty() {
        return false;
    }
    if seq.blocks[i].kind == k::EOF {
        let changed = p.newlines == 0 && !p.ws.is_empty();
        if changed {
            p.set_ws("");
        }
        return changed;
    }
    if p.newlines > 0 && !strip {
        return false;
    }
    let had_break = p.newlines > 0;
    p.newlines = 0;
    if had_break {
        p.set_ws("");
    }
    let want = if any || (touch && fuse) {
        None
    } else {
        Some(if touch { "" } else { " " })
    };
    match want {
        Some(w) if p.ws != w => {
            p.set_ws(w);
            true
        }
        _ => had_break,
    }
}
