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
    /// `name@vN` of an action: outdated when the table lists the name with a last outdated
    /// major version of N or more.
    Outdated(&'static [(&'static str, u32)]),
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

impl Format {
    /// The problem of `text`, if it is not of this format.
    pub fn check(self, text: &str) -> Option<String> {
        match self {
            Format::None => None,
            Format::Outdated(table) => {
                let (name, version) = text.split_once('@')?;
                let major = version.strip_prefix('v')?.parse::<u32>().ok()?;
                let (_, last) = table.iter().find(|(n, _)| n.eq_ignore_ascii_case(name))?;
                (major <= *last).then(|| {
                    format!("the runner of \"{text}\" action is too old to run on GitHub Actions. update the action's version to fix this warning")
                })
            }
            Format::Positive => match text.parse::<f64>() {
                Ok(n) if n <= 0.0 => Some(format!("value must be greater than zero: {text}")),
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
                None
            }
        }
    }
}
