//! Guards around the work of one file: a file that cannot be handled becomes one failure and the
//! run goes on with the other files.
//!
//! - [`check_nesting`] runs before every parse. Parsers written by others (oxc, ruff, mago,
//!   tree-sitter walks) recurse on the stack of the calling thread, and a stack overflow
//!   cannot be caught, so the limit has to come before the parser. It is language-neutral: the
//!   deepest nesting of `(`, `[` and `{` in the text, strings and comments included (a closer
//!   with nothing open is ignored). Code deeper than [`MAX_NESTING`] does not exist outside
//!   generated or hostile input.
//! - [`catch`] turns a panic of a parser, formatter or rule into a message; the engine's queries
//!   catch around their whole body (the engine's own cancellation unwind passes through).
//!
//! concept: parse-guards
//! what: Nesting, size and panic guards around the parse of one file.
//! tools: none (robustness)
//! reads: grammar:language

use crate::provider::{LanguageProvider, ParseResult, ProviderError, SourceFile};
use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};

/// The deepest bracket nesting a file may have.
pub const MAX_NESTING: usize = 1000;

/// Whether `text` nests brackets deeper than [`MAX_NESTING`]; the message names the position of
/// the first bracket past the limit.
///
/// Most text is never near the limit, so it is read in blocks of 128 bytes by counting
/// brackets, which the compiler vectorizes (12 GB/s, ten times faster than reading the bytes in
/// turn). `bound` is an upper bound of the depth at the end of a block: the depth before it plus
/// its net count, or its openers if a closer met an empty depth (`max(bound + opens - closes,
/// opens)`). Only a block that could pass the limit is read byte by byte, from the last place
/// the depth was known exactly.
pub fn check_nesting(text: &str) -> Result<(), String> {
    const BLOCK: usize = 128;
    let bytes = text.as_bytes();
    let (mut bound, mut known_at, mut known) = (0usize, 0usize, 0usize);
    for (n, block) in bytes.chunks(BLOCK).enumerate() {
        // Byte counters: a block is under 256 bytes, and eight bits per lane vectorize best.
        let opens = block
            .iter()
            .fold(0u8, |n, &b| n + matches!(b, b'(' | b'[' | b'{') as u8)
            as usize;
        let closes = block
            .iter()
            .fold(0u8, |n, &b| n + matches!(b, b')' | b']' | b'}') as u8)
            as usize;
        if bound + opens <= MAX_NESTING {
            bound = (bound + opens).saturating_sub(closes).max(opens);
            continue;
        }
        let end = (n * BLOCK + block.len()).min(bytes.len());
        let mut depth = known;
        for (i, &b) in bytes[known_at..end].iter().enumerate() {
            match b {
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => depth = depth.saturating_sub(1),
                _ => {}
            }
            if depth > MAX_NESTING {
                return Err(format!(
                    "nesting is deeper than {MAX_NESTING} levels (at byte {})",
                    known_at + i
                ));
            }
        }
        (bound, known_at, known) = (depth, end, depth);
    }
    Ok(())
}

/// `provider.parse(file)` after the nesting check and with panics caught: the entry every
/// engine query parses through.
pub fn parse(
    provider: &dyn LanguageProvider,
    file: &SourceFile,
) -> Result<ParseResult, ProviderError> {
    check_nesting(&file.text).map_err(ProviderError::Unparsable)?;
    match catch(|| provider.parse(file)) {
        Ok(result) => result,
        Err(message) => Err(ProviderError::Unparsable(message)),
    }
}

/// Runs `work`, turning a panic into its message.
pub fn catch<T>(work: impl FnOnce() -> T) -> Result<T, String> {
    catch_unwind(AssertUnwindSafe(work)).map_err(|payload| panic_message(&*payload))
}

/// What a panic said, as a failure message.
pub fn panic_message(payload: &(dyn Any + Send)) -> String {
    let text = payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&str>().copied())
        .unwrap_or("unknown cause");
    format!("internal error (a panic): {text}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nesting_counts_open_brackets_and_ignores_stray_closers() {
        let nested = |n: usize| format!("{}x", "([{".repeat(n / 3));
        assert!(check_nesting(&nested(MAX_NESTING)).is_ok());
        assert!(check_nesting(&nested(MAX_NESTING + 3)).is_err());
        assert!(check_nesting(&format!("{}{}", ")".repeat(5000), "(".repeat(MAX_NESTING))).is_ok());
        assert!(check_nesting(&"()".repeat(50_000)).is_ok());
        let wave = format!("{}{}", "(".repeat(900), ")".repeat(900));
        assert!(check_nesting(&wave.repeat(20)).is_ok());
        let late = format!(
            "{}{}{}",
            "x".repeat(1000),
            "[".repeat(1001),
            "]".repeat(1001)
        );
        assert!(
            check_nesting(&late)
                .unwrap_err()
                .ends_with("(at byte 2000)")
        );
        assert!(check_nesting(&format!("{wave}{}{wave}", "{".repeat(101))).is_err());
    }

    /// `cargo test --release -p provider-api --lib -- --ignored nesting_cost --nocapture`
    #[test]
    #[ignore = "a measurement"]
    fn nesting_cost() {
        let chunk = "function f(a, b) { return [a(b), {x: (1 + 2)}]; }\n// comment text\n";
        let text = chunk.repeat(200_000);
        let best = (0..10)
            .map(|_| {
                let start = std::time::Instant::now();
                assert!(check_nesting(std::hint::black_box(&text)).is_ok());
                start.elapsed()
            })
            .min()
            .unwrap();
        let mb = text.len() as f64 / 1e6;
        println!(
            "{mb:.1} MB in {best:?}: {:.0} MB/s",
            mb / best.as_secs_f64()
        );
    }

    #[test]
    fn a_panic_is_a_message() {
        let caught = catch(|| -> u8 { panic!("boom") });
        assert_eq!(caught, Err("internal error (a panic): boom".into()));
    }
}
