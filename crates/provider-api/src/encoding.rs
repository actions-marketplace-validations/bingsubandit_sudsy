//! Source bytes that are not UTF-8: a UTF-16 file (byte-order mark) and a single-byte file (Windows-1252, where
//! every byte maps to one code point, so the text round-trips byte for byte). The text of a source is always
//! UTF-8; the writers encode the result back in the encoding the file had.
//!
//! concept: none (infrastructure: input encoding)

/// Code points of the bytes 0x80..=0x9F in Windows-1252; the five undefined bytes map to the C1 control of the same value.
const CP1252: [char; 32] = [
    '\u{20ac}', '\u{81}', '\u{201a}', '\u{192}', '\u{201e}', '\u{2026}', '\u{2020}', '\u{2021}',
    '\u{2c6}', '\u{2030}', '\u{160}', '\u{2039}', '\u{152}', '\u{8d}', '\u{17d}', '\u{8f}',
    '\u{90}', '\u{2018}', '\u{2019}', '\u{201c}', '\u{201d}', '\u{2022}', '\u{2013}', '\u{2014}',
    '\u{2dc}', '\u{2122}', '\u{161}', '\u{203a}', '\u{153}', '\u{9d}', '\u{17e}', '\u{178}',
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Utf16(bool),
    SingleByte,
}

fn kind_of(bytes: &[u8]) -> Option<Kind> {
    match bytes {
        [0xff, 0xfe, ..] => Some(Kind::Utf16(true)),
        [0xfe, 0xff, ..] => Some(Kind::Utf16(false)),
        _ if std::str::from_utf8(bytes).is_ok() => None,
        _ => Some(Kind::SingleByte),
    }
}

/// The text of bytes that are not valid UTF-8 (the byte-order mark of UTF-16 dropped), `None` for a lone surrogate.
pub fn decode(bytes: &[u8]) -> Option<String> {
    match kind_of(bytes)? {
        Kind::Utf16(le) => {
            let units = bytes[2..].chunks(2).map(|c| match *c {
                [a, b] if le => u16::from_le_bytes([a, b]),
                [a, b] => u16::from_be_bytes([a, b]),
                _ => 0xd800,
            });
            char::decode_utf16(units)
                .collect::<Result<String, _>>()
                .ok()
        }
        Kind::SingleByte => Some(
            bytes
                .iter()
                .map(|&b| match b {
                    0x80..=0x9f => CP1252[usize::from(b) - 0x80],
                    _ => char::from(b),
                })
                .collect(),
        ),
    }
}

/// Encodes `text` like `original`, the bytes the file had: UTF-16 with its mark, Windows-1252, else UTF-8 (with the
/// mark when `had_bom`). `None` when a character has no byte in the encoding of the file.
pub fn encode_like(original: &[u8], text: &str, had_bom: bool) -> Option<Vec<u8>> {
    match kind_of(original) {
        Some(Kind::Utf16(le)) => {
            let unit = |u: u16| if le { u.to_le_bytes() } else { u.to_be_bytes() };
            Some(
                std::iter::once(0xfeff)
                    .chain(text.encode_utf16())
                    .flat_map(unit)
                    .collect(),
            )
        }
        Some(Kind::SingleByte) => text
            .chars()
            .map(|c| match u32::from(c) {
                b @ (0..0x80 | 0xa0..0x100) => Some(b as u8),
                _ => CP1252.iter().position(|&x| x == c).map(|i| 0x80 + i as u8),
            })
            .collect(),
        None => {
            let mut out = Vec::with_capacity(text.len() + 3);
            if had_bom {
                out.extend_from_slice("\u{feff}".as_bytes());
            }
            out.extend_from_slice(text.as_bytes());
            Some(out)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_byte_text_round_trips() {
        let bytes: Vec<u8> = (0x80..=0xffu8).chain(*b"\r\nselect").collect();
        let text = decode(&bytes).unwrap();
        assert!(text.starts_with("\u{20ac}\u{81}"));
        assert_eq!(encode_like(&bytes, &text, false).unwrap(), bytes);
    }

    #[test]
    fn utf16_round_trips_with_its_mark() {
        let mut bytes = vec![0xff, 0xfe];
        bytes.extend("select 1\u{e9}".encode_utf16().flat_map(u16::to_le_bytes));
        assert_eq!(decode(&bytes).unwrap(), "select 1\u{e9}");
        assert_eq!(
            encode_like(&bytes, "select 2", false).unwrap()[..4],
            [0xff, 0xfe, b's', 0]
        );
        assert!(decode(&[0xff, 0xfe, 0x00, 0xd8]).is_none());
    }

    #[test]
    fn utf8_is_left_to_the_caller() {
        assert!(decode(b"select 1").is_none());
        assert_eq!(encode_like(b"x", "a", true).unwrap(), b"\xef\xbb\xbfa");
    }
}
