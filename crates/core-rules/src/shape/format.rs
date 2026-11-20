//! Text formats a scalar of a shape row can be required to have.
//!
//! concept: none (infrastructure: shape formats)

/// A format of a scalar value.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Format {
    None,
    /// A five-field cron line.
    Cron,
    /// A number above zero.
    Positive,
    /// A spec (`owner/repo@ref`) of an action listed, one per line, as outdated.
    Outdated(&'static str),
    /// The text must be a line of the list; the message takes `{text}`.
    Listed(&'static str, &'static str),
    /// A filter pattern for branch and tag names, or for paths (the glob syntax of GitHub's filters).
    RefGlob,
    PathGlob,
    /// A name that starts with a letter or `_` and has only letters, digits, `-` and `_`.
    Identifier,
    /// The value must be an expression (a secret), not written out.
    Expression,
}

const MONTHS: &[&str] = &[
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];
const DAYS: &[&str] = &["sun", "mon", "tue", "wed", "thu", "fri", "sat"];

/// One cron field: a list of `*`, numbers, names and ranges, each with an optional `/step`.
fn cron_field(
    field: &str,
    min: u32,
    max: u32,
    names: &[&str],
    first_name: u32,
) -> Result<(), String> {
    let number = |s: &str| -> Option<u32> {
        let n = match s.parse::<u32>() {
            Ok(n) => n,
            Err(_) => {
                first_name
                    + u32::try_from(names.iter().position(|n| n.eq_ignore_ascii_case(s))?).ok()?
            }
        };
        (min..=max).contains(&n).then_some(n)
    };
    for part in field.split(',') {
        let (range, step) = match part.split_once('/') {
            Some((r, s)) => (r, Some(s)),
            None => (part, None),
        };
        if let Some(step) = step
            && !step.parse::<u32>().is_ok_and(|n| n > 0)
        {
            return Err(format!("invalid step \"{step}\" in \"{part}\""));
        }
        if range == "*" {
            continue;
        }
        let bounds: Vec<&str> = range.split('-').collect();
        let ok = match bounds.as_slice() {
            [one] => number(one).is_some(),
            [a, b] => matches!((number(a), number(b)), (Some(a), Some(b)) if a <= b),
            _ => false,
        };
        if !ok {
            return Err(format!("\"{part}\" is out of range {min}-{max}"));
        }
    }
    Ok(())
}

/// The values a numeric cron field (`*`, `a`, `a-b`, each with `/step`, joined by `,`) selects.
fn cron_values(field: &str, min: u32, max: u32) -> Vec<u32> {
    let mut out = Vec::new();
    for part in field.split(',') {
        let (range, step) = part.split_once('/').unwrap_or((part, "1"));
        let step = step.parse::<u32>().unwrap_or(1).max(1);
        let (lo, hi) = match range.split_once('-') {
            _ if range == "*" => (min, max),
            Some((a, b)) => (a.parse().unwrap_or(min), b.parse().unwrap_or(max)),
            None => {
                let a = range.parse().unwrap_or(min);
                (a, if part.contains('/') { max } else { a })
            }
        };
        out.extend((lo..=hi).step_by(step as usize));
    }
    out
}

/// The shortest gap in seconds between two runs of a cron line, from its minute and hour fields.
fn cron_interval(minutes: &str, hours: &str) -> u32 {
    let mut runs: Vec<u32> = Vec::new();
    for h in cron_values(hours, 0, 23) {
        runs.extend(cron_values(minutes, 0, 59).into_iter().map(|m| h * 60 + m));
    }
    runs.sort_unstable();
    runs.dedup();
    let wrap = runs.first().map_or(1440, |f| f + 1440);
    let last = runs.last().copied().unwrap_or(0);
    runs.windows(2)
        .map(|w| w[1] - w[0])
        .chain([wrap - last])
        .min()
        .unwrap_or(1440)
        * 60
}

impl Format {
    /// The problem of `text`, if it is not of this format.
    pub fn check(self, text: &str) -> Option<String> {
        match self {
            Format::None => None,
            Format::Listed(list, message) => (!list.lines().any(|l| l == text))
                .then(|| message.replace("{text}", text)),
            Format::Outdated(specs) => specs.lines().any(|l| l == text).then(|| {
                format!("the runner of \"{text}\" action is too old to run on GitHub Actions. update the action's version to fix this issue")
            }),
            Format::Identifier => {
                let mut chars = text.chars();
                let start = chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
                (!(start && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_')))).then(|| {
                    format!("invalid ID \"{text}\". an ID must start with a letter or _ and contain only alphanumeric characters, -, or _")
                })
            }
            Format::Expression => None,
            Format::RefGlob => glob(text, false),
            Format::PathGlob => glob(text, true),
            Format::Positive => match text.parse::<f64>() {
                Ok(n) if n <= 0.0 => Some(format!("value at \"{{name}}\" must be greater than zero: {text}")),
                _ => None,
            },
            Format::Cron => {
                let fields: Vec<&str> = text.split_whitespace().collect();
                if fields.len() != 5 {
                    return Some(format!(
                        "invalid CRON format \"{text}\" in schedule event: expected exactly 5 fields, found {}",
                        fields.len()
                    ));
                }
                let specs: [(u32, u32, &[&str], u32); 5] = [
                    (0, 59, &[], 0),
                    (0, 23, &[], 0),
                    (1, 31, &[], 0),
                    (1, 12, MONTHS, 1),
                    (0, 7, DAYS, 0),
                ];
                for (field, (min, max, names, first)) in fields.iter().zip(specs) {
                    if let Err(e) = cron_field(field, min, max, names, first) {
                        return Some(format!(
                            "invalid CRON format \"{text}\" in schedule event: {e}"
                        ));
                    }
                }
                let seconds = cron_interval(fields[0], fields[1]);
                (seconds < 300).then(|| {
                    format!("scheduled job runs too frequently. it runs once per {seconds} seconds. the shortest interval is once every 5 minutes")
                })
            }
        }
    }
}

const SPECIAL: &str = "[?+*\\!";

/// The problem of a filter pattern (the glob syntax of GitHub's branch, tag and path filters).
fn glob(text: &str, path: bool) -> Option<String> {
    let note = "note: filter pattern syntax is explained at https://docs.github.com/en/actions/using-workflows/workflow-syntax-for-github-actions#filter-pattern-cheat-sheet";
    let body = match text.strip_prefix('!') {
        Some("") => {
            return Some(format!(
                "invalid glob pattern. unexpected end after `!` at first character (negate pattern): at least one character must follow. {note}"
            ));
        }
        Some(rest) => rest,
        None => text,
    };
    if path {
        if body != body.trim() {
            return Some(format!(
                "leading and trailing spaces are not allowed in glob path. {note}"
            ));
        }
        if body.split('/').any(|s| s == "." || s == "..") {
            return Some(format!("'.' and '..' are not allowed in glob path. {note}"));
        }
    } else {
        if body.starts_with('/') || body.ends_with('/') || body.ends_with('.') {
            return Some(format!(
                "a ref name must not start with / or end with / or .: \"{body}\". {note}"
            ));
        }
        let mut chars = body.chars();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some(n) if SPECIAL.contains(n) => {}
                    _ => {
                        return Some(format!(
                            "character '\\' is invalid for branch and tag names. only special characters [, ?, +, *, \\, ! can be escaped with \\. {note}"
                        ));
                    }
                }
            } else if matches!(c, ' ' | '~' | '^' | ':') {
                return Some(format!(
                    "character '{c}' is invalid for branch and tag names. ref name cannot contain spaces, ~, ^, :. {note}"
                ));
            }
        }
    }
    let chars: Vec<char> = body.chars().collect();
    let (mut i, mut quantifiable) = (0, false);
    while i < chars.len() {
        match chars[i] {
            '\\' => {
                i += 1;
                quantifiable = true;
            }
            '*' => quantifiable = false,
            '?' | '+' => {
                if !quantifiable {
                    return Some(format!(
                        "invalid glob pattern. unexpected character '{}' while checking special character: the preceding character must be a normal one. {note}",
                        chars[i]
                    ));
                }
                quantifiable = false;
            }
            '[' => {
                let mut j = i + 1;
                let mut class = Vec::new();
                while j < chars.len() && chars[j] != ']' {
                    class.push(chars[j]);
                    j += 1;
                }
                if j >= chars.len() {
                    return Some(format!(
                        "invalid glob pattern. unexpected EOF while checking end of character match []. missing ]. {note}"
                    ));
                }
                if class.len() == 1 {
                    return Some(format!(
                        "invalid glob pattern. unexpected character ']' while checking character match []: a character match needs more than one character. {note}"
                    ));
                }
                let mut k = 0;
                while k + 2 < class.len() {
                    if class[k + 1] == '-' && class[k] > class[k + 2] {
                        return Some(format!(
                            "invalid glob pattern. start of range '{}' is larger than end of range '{}'. {note}",
                            class[k],
                            class[k + 2]
                        ));
                    }
                    k += if class[k + 1] == '-' { 3 } else { 1 };
                }
                i = j;
                quantifiable = true;
            }
            _ => quantifiable = true,
        }
        i += 1;
    }
    None
}
