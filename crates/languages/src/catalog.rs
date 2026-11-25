//! The parity catalog (`docs/ai/parity/*.yaml`), embedded: the source of the reasons printed for
//! settings that cannot be migrated, of the checks that each `init --from` mapping agrees with it,
//! and of the "Equivalent" ids shown for each rule.

use serde::Deserialize;
use std::sync::OnceLock;

pub struct Entry {
    pub id: String,
    /// `rule: <id>`, `option: <name>`, `formatter`, `sdk: <id>`, `config: <text>`, `planned`,
    /// `deferred` or `wontfix: <reason>`.
    pub sudsy: String,
    /// `covered`, `partial`, `missing`, `sdk`, `wontfix` or `deferred`.
    pub coverage: String,
    pub gap: String,
    /// The tool's page for this rule (see [`Catalog::rule_url`]), empty when it has none.
    pub url: String,
}

impl Entry {
    /// The Sudsy rule ids of a `rule: <id>[, <id>...]` status (several when one upstream rule is
    /// split over ids), empty for any other status.
    pub fn rules(&self) -> impl Iterator<Item = &str> {
        self.sudsy
            .strip_prefix("rule: ")
            .into_iter()
            .flat_map(|ids| ids.split(", "))
    }
}

#[derive(Deserialize)]
pub struct Catalog {
    pub tool: String,
    pub languages: Vec<String>,
    /// Where a rule of the tool is documented: a URL with `{id}`, `{id-lower}`, `{id-flat}` (lowercase
    /// letters and digits of the id), `{group}` (the lowercase part before the first `/`) and `{name}`
    /// (the part after it, or the whole id), or `none` for a tool without any page for its rules.
    /// A rule with no page of its own uses the anchor of the list that names it.
    #[serde(rename = "rule-url")]
    pub rule_url: String,
    /// A prefix of the ids that the template leaves out (`PS` of `PSUseSingularNouns`).
    #[serde(rename = "rule-url-strip", default)]
    rule_url_strip: String,
    /// Pages that do not fit the template, by entry id.
    #[serde(rename = "rule-urls", default)]
    rule_urls: std::collections::BTreeMap<String, String>,
    /// Filled by [`parse`] from `listed` and `deferred`.
    #[serde(skip)]
    pub entries: Vec<Entry>,
    #[serde(rename = "entries")]
    listed: Pairs,
    #[serde(default)]
    deferred: Vec<Deferred>,
}

#[derive(Deserialize)]
struct Deferred {
    gap: String,
    ids: Vec<String>,
}

/// A YAML map as its pairs in file order.
#[derive(Default)]
struct Pairs(Vec<(String, String)>);

impl<'de> Deserialize<'de> for Pairs {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> serde::de::Visitor<'de> for V {
            type Value = Pairs;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a map of id to status")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(self, mut m: A) -> Result<Pairs, A::Error> {
                let mut out = Vec::new();
                while let Some(pair) = m.next_entry()? {
                    out.push(pair);
                }
                Ok(Pairs(out))
            }
        }
        d.deserialize_map(V)
    }
}

/// Reads a catalog file (format: docs/ai/parity/README.md): each `id: <status> [| <gap>]` of
/// `entries`, then each id under `deferred` with the group's `gap`.
pub fn parse(text: &str) -> Result<Catalog, String> {
    let mut c: Catalog = serde_saphyr::from_str(text).map_err(|e| e.to_string())?;
    for (id, value) in std::mem::take(&mut c.listed).0 {
        let (status, gap) = value.split_once(" | ").unwrap_or((&value, ""));
        let (kind, arg) = status.split_once(' ').unwrap_or((status, ""));
        let coverage = match kind {
            "sdk" | "wontfix" | "deferred" => kind,
            "planned" => "missing",
            _ if gap.is_empty() => "covered",
            _ => "partial",
        };
        let sudsy = if arg.is_empty() {
            kind.into()
        } else {
            format!("{kind}: {arg}")
        };
        c.entries.push(Entry {
            id,
            sudsy,
            coverage: coverage.into(),
            gap: gap.into(),
            url: String::new(),
        });
    }
    for d in std::mem::take(&mut c.deferred) {
        c.entries.extend(d.ids.into_iter().map(|id| Entry {
            id,
            sudsy: "deferred".into(),
            coverage: "deferred".into(),
            gap: d.gap.clone(),
            url: String::new(),
        }));
    }
    for id in c.rule_urls.keys() {
        if !c.entries.iter().any(|e| &e.id == id) {
            return Err(format!("rule-urls: `{id}` is no entry"));
        }
    }
    for e in &mut c.entries {
        // An option, a formatter behaviour or a setting is not a rule: no page of a rule list names it.
        if matches!(
            e.sudsy.split(':').next(),
            Some("option" | "formatter" | "config")
        ) {
            continue;
        }
        e.url = match c.rule_urls.get(&e.id) {
            Some(url) => url.clone(),
            None if c.rule_url == "none" => String::new(),
            None => resolve(
                &c.rule_url,
                e.id.strip_prefix(&c.rule_url_strip).unwrap_or(&e.id),
            )?,
        };
    }
    Ok(c)
}

/// A `rule-url` template for one entry id.
fn resolve(template: &str, id: &str) -> Result<String, String> {
    let (group, name) = id.split_once('/').unwrap_or(("", id));
    let flat: String = id
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect();
    let mut out = String::new();
    let mut rest = template;
    while let Some((head, tail)) = rest.split_once('{') {
        let (var, after) = tail
            .split_once('}')
            .ok_or_else(|| format!("rule-url `{template}`: unclosed `{{`"))?;
        out += head;
        out += &match var {
            "id" => id.to_string(),
            "id-lower" => id.to_lowercase(),
            "id-flat" => flat.clone(),
            "group" => group.to_lowercase(),
            "name" => name.to_string(),
            _ => return Err(format!("rule-url `{template}`: unknown `{{{var}}}`")),
        };
        rest = after;
    }
    Ok(out + rest)
}

macro_rules! catalogs {
    ($($name:literal),*) => {{
        static CELL: OnceLock<Vec<Catalog>> = OnceLock::new();
        CELL.get_or_init(|| {
            vec![$(parse(include_str!(concat!(
                "../../../docs/ai/parity/", $name, ".yaml"
            )))
            .expect("parity catalog parses")),*]
        })
    }};
}

/// Every catalog, one per tool, in file name order.
pub fn all() -> &'static [Catalog] {
    catalogs!(
        "biome",
        "csharpier",
        "dotnet-format-editorconfig",
        "eslint",
        "eslint-plugin-jsonc",
        "html-eslint",
        "html-validate",
        "htmlhint",
        "prettier",
        "roslynator",
        "shfmt",
        "stylecop",
        "stylelint",
        "stylelint-less",
        "stylelint-scss",
        "taplo",
        "typescript-eslint",
        "yamllint"
    )
}

/// The entries of the catalog named `tool` (the file name without `.yaml`).
pub fn entries(tool: &str) -> &'static [Entry] {
    all()
        .iter()
        .find(|c| c.tool == tool)
        .map_or(&[], |c| &c.entries)
}

/// The entry for an upstream setting. Some entries name several settings (`a / b`, `a, b`).
pub fn find(tool: &str, id: &str) -> Option<&'static Entry> {
    // A tool's plugin catalogs (`stylelint-scss` for `stylelint`) are searched with its own.
    all()
        .iter()
        .filter(|c| {
            c.tool == tool
                || c.tool
                    .strip_prefix(tool)
                    .is_some_and(|r| r.starts_with('-'))
        })
        .flat_map(|c| &c.entries)
        .find(|e| e.id == id || e.id.split([',', '/']).any(|part| part.trim() == id))
}

/// Why a setting cannot be migrated, in the catalog's words.
pub fn reason(tool: &str, id: &str) -> String {
    match find(tool, id) {
        Some(e) if !e.gap.is_empty() => e.gap.clone(),
        Some(e) if e.sudsy == "formatter" => "fixed formatter behavior, not configurable".into(),
        Some(e) if e.sudsy.starts_with("rule: ") || e.sudsy.starts_with("option: ") => {
            format!("this setting has no Sudsy option; {}", e.sudsy)
        }
        Some(e) => e.sudsy.clone(),
        None => "not in the parity catalog".into(),
    }
}

fn tool_name(tool: &str) -> &str {
    match tool {
        "stylecop" => "StyleCop",
        "csharpier" => "CSharpier",
        "roslynator" => "Roslynator",
        "prettier" => "Prettier",
        "biome" => "Biome",
        other => other,
    }
}

/// An upstream setting that maps to a rule.
pub struct Upstream {
    /// The tool's display name ("StyleCop").
    pub tool: String,
    pub id: String,
    pub partial: bool,
    /// The tool's page for the rule, empty when it has none.
    pub url: String,
    /// What differs of a partial match, in the catalog's words.
    pub note: String,
}

impl std::fmt::Display for Upstream {
    /// "StyleCop SA1106", with " (partial)" for partial coverage.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let partial = if self.partial { " (partial)" } else { "" };
        write!(f, "{} {}{partial}", self.tool, self.id)
    }
}

/// The upstream settings that map to the rule `rule` in one of `languages` (`all`: every
/// language), in catalog order.
pub fn upstream(rule: &str, languages: &[&str]) -> Vec<Upstream> {
    upstream_in(all(), rule, languages)
}

/// [`upstream`] over the given catalogs (the docs generator reads every file of `docs/ai/parity`).
pub fn upstream_in(catalogs: &[Catalog], rule: &str, languages: &[&str]) -> Vec<Upstream> {
    let mut out = Vec::new();
    for c in catalogs {
        let applies = languages.contains(&"all")
            || c.languages.iter().any(|l| languages.contains(&l.as_str()));
        if !applies {
            continue;
        }
        for e in &c.entries {
            if e.rules().any(|r| r == rule) && matches!(e.coverage.as_str(), "covered" | "partial")
            {
                out.push(Upstream {
                    tool: tool_name(&c.tool).to_string(),
                    id: e.id.clone(),
                    partial: e.coverage == "partial",
                    url: e.url.clone(),
                    note: e.gap.clone(),
                });
            }
        }
    }
    out
}

/// The rules that cover the upstream setting `id` (case-insensitive; the whole id or one of the
/// settings an entry names).
pub fn rules_for(id: &str) -> Vec<&'static str> {
    let mut out: Vec<&str> = all()
        .iter()
        .flat_map(|c| &c.entries)
        .filter(|e| matches!(e.coverage.as_str(), "covered" | "partial"))
        .filter(|e| {
            e.id.eq_ignore_ascii_case(id)
                || e.id
                    .split([',', '/'])
                    .any(|p| p.trim().eq_ignore_ascii_case(id))
        })
        .flat_map(Entry::rules)
        .collect();
    out.sort();
    out.dedup();
    out
}
