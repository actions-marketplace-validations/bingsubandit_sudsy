//! The word convention of `naming-convention` (`words`): a name is words, written without
//! underscores, each word after the first starting with a capital, and a word that is an
//! initialism (`ID`, `HTTP`, from the `initialisms` option) in capitals (`userID`, `HTTPServer`).
//! A name of capitals and underscores longer than five bytes is a constant written in another
//! language's style.
//!
//! concept: none (helper of naming-convention: the spelling of a name in words)

/// How `name` should be spelled when it does not follow the convention; `None` when it does.
/// `initialisms` are the words (in capitals) that are written in capitals.
pub fn suggest(name: &str, initialisms: &[&str]) -> Option<String> {
    if name == "_" || name.chars().all(char::is_lowercase) {
        return None;
    }
    let should = if all_caps(name) {
        pascal(name)
    } else {
        spell(name, initialisms)
    };
    (should != name || all_caps(name)).then_some(should)
}

/// `FOO_BAR_BAZ`: capitals, digits and underscores, with an underscore and more than five bytes.
fn all_caps(name: &str) -> bool {
    name.contains('_')
        && name.len() > 5
        && name
            .chars()
            .all(|c| c == '_' || c.is_uppercase() || c.is_ascii_digit())
}

fn pascal(name: &str) -> String {
    name.split('_')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            let first = chars.next().map(|c| c.to_uppercase().to_string());
            first.unwrap_or_default() + &chars.as_str().to_lowercase()
        })
        .collect()
}

/// Splits at every lower to non-lower step and at underscores (which go), writes initialisms in
/// capitals (lowercase at the start of the name) and starts every other later word with a capital.
fn spell(name: &str, initialisms: &[&str]) -> String {
    let mut runes: Vec<char> = name.chars().collect();
    let (mut w, mut i) = (0, 0);
    while i < runes.len() {
        let mut end = false;
        if i + 1 == runes.len() {
            end = true;
        } else if runes[i + 1] == '_' {
            end = true;
            let mut n = 1;
            while i + n + 1 < runes.len() && runes[i + n + 1] == '_' {
                n += 1;
            }
            // one underscore stays between two digits
            if i + n + 1 < runes.len()
                && runes[i].is_ascii_digit()
                && runes[i + n + 1].is_ascii_digit()
            {
                n -= 1;
            }
            runes.drain(i + 1..i + 1 + n);
        } else if runes[i].is_lowercase() && !runes[i + 1].is_lowercase() {
            end = true;
        }
        i += 1;
        if !end {
            continue;
        }
        let word: String = runes[w..i].iter().collect();
        let upper = word.to_uppercase();
        if initialisms.contains(&upper.as_str()) && upper.chars().count() == i - w {
            let mut spelled = if w == 0 && runes[w].is_lowercase() {
                upper.to_lowercase()
            } else {
                upper
            };
            if spelled == "IDS" {
                spelled = "IDs".to_string();
            }
            for (at, c) in spelled.chars().enumerate() {
                runes[w + at] = c;
            }
        } else if w > 0 && word.to_lowercase() == word {
            let first: Vec<char> = runes[w].to_uppercase().collect();
            if let [c] = first[..] {
                runes[w] = c;
            }
        }
        w = i;
    }
    runes.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const INITIALISMS: &[&str] = &["API", "HTTP", "ID", "IDS", "URL"];

    fn s(name: &str) -> Option<String> {
        suggest(name, INITIALISMS)
    }

    #[test]
    fn words() {
        assert_eq!(s("foo_bar").as_deref(), Some("fooBar"));
        assert_eq!(s("userId").as_deref(), Some("userID"));
        assert_eq!(s("Id").as_deref(), Some("ID"));
        assert_eq!(s("startHttpServer").as_deref(), Some("startHTTPServer"));
        assert_eq!(s("DW_TAG_array_type").as_deref(), Some("DWTAGArrayType"));
        assert_eq!(s("type_").as_deref(), Some("type"));
        assert_eq!(s("v1_2"), None);
        assert_eq!(s("userIDs"), None);
        assert_eq!(s("fooBar"), None);
        assert_eq!(s("_"), None);
        assert_eq!(s("_foo"), None);
    }

    #[test]
    fn all_caps_names() {
        assert_eq!(s("MAX_VALUE").as_deref(), Some("MaxValue"));
        assert_eq!(s("A_B").as_deref(), Some("AB"));
    }
}
