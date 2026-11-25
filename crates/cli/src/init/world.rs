//! What `sudsy init` learns before it asks: the languages in the working directory, the presets
//! the binary has, and the configs of other tools that can be migrated.

use super::out::Out;
use super::{NotFound, Tool, migrate};
use anyhow::Result;
use clap::ValueEnum;
use provider_api::{FilePattern, LanguageProvider};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// The scan stops at this many files or this much time, whichever comes first.
const MAX_FILES: usize = 20_000;
const MAX_TIME: Duration = Duration::from_secs(2);

pub struct Lang {
    pub name: String,
    pub files: usize,
    /// Globs that select the files of this language (from its file matchers).
    pub globs: Vec<String>,
    /// VS Code language ids of its crate.
    pub vscode: Vec<String>,
}

pub struct PresetInfo {
    pub name: String,
    pub description: String,
    pub text: &'static str,
    /// Sets formatting options (a style); otherwise it only sets rule levels.
    pub style: bool,
}

pub struct Found {
    pub tool: Tool,
    pub name: String,
    pub source: String,
    pub out: Out,
}

pub struct World {
    pub langs: Vec<Lang>,
    /// (path, index into `langs`) of every scanned file, for checking exclusion globs.
    pub files: Vec<(PathBuf, usize)>,
    pub capped: bool,
    pub presets: Vec<PresetInfo>,
    pub tools: Vec<Found>,
    /// Configs that exist but cannot be read, each with what to do.
    pub problems: Vec<String>,
    pub agent_files: [bool; 2],
    pub vscode_dir: bool,
    pub github: bool,
}

impl World {
    /// `dir` scanned for languages; `tools` also looks for the configs of other tools in the
    /// working directory.
    pub fn load(dir: &Path, providers: &[Box<dyn LanguageProvider>], tools: bool) -> Result<World> {
        let (langs, files, capped) = scan(dir, providers)?;
        let (tools, problems) = if tools { detect() } else { Default::default() };
        let github = dir.join(".github").is_dir()
            || std::fs::read_to_string(dir.join(".git/config"))
                .is_ok_and(|c| c.contains("github.com"));
        Ok(World {
            langs,
            files,
            capped,
            presets: presets(providers),
            tools,
            problems,
            agent_files: ["AGENTS.md", "CLAUDE.md"].map(|n| dir.join(n).exists()),
            vscode_dir: dir.join(".vscode").is_dir(),
            github,
        })
    }

    /// Only the presets: the working directory is not read.
    pub fn bare(providers: &[Box<dyn LanguageProvider>]) -> World {
        World {
            langs: Vec::new(),
            files: Vec::new(),
            capped: false,
            presets: presets(providers),
            tools: Vec::new(),
            problems: Vec::new(),
            agent_files: [false; 2],
            vscode_dir: false,
            github: false,
        }
    }

    /// The style preset a tool's output extends for `language`, if any (rule-only presets do
    /// not decide the style).
    pub fn own_style<'a>(&self, out: &'a Out, language: &'a str) -> Option<&'a str> {
        out.presets_of(language)
            .find(|p| self.preset(p).is_some_and(|i| i.style))
    }

    /// The languages `out` sets something for that this repository has and that stay in the
    /// config (all of them when nothing was scanned).
    pub fn covered(
        &self,
        out: &Out,
        deselected: &std::collections::BTreeSet<String>,
    ) -> Vec<String> {
        out.languages()
            .into_iter()
            .filter(|l| !deselected.contains(l))
            .filter(|l| self.langs.is_empty() || self.langs.iter().any(|x| &x.name == l))
            .collect()
    }

    pub fn preset(&self, name: &str) -> Option<&PresetInfo> {
        self.presets.iter().find(|p| p.name == name)
    }

    /// The presets of a language, style or rule-only, in the order the language lists them.
    pub fn presets_of<'a>(
        &'a self,
        language: &'a str,
        style: bool,
    ) -> impl Iterator<Item = &'a PresetInfo> {
        self.presets
            .iter()
            .filter(move |p| super::out::language_of(&p.name) == language && p.style == style)
    }
}

/// The presets of every provider, each once (a dialect lists the presets of its base).
fn presets(providers: &[Box<dyn LanguageProvider>]) -> Vec<PresetInfo> {
    let mut seen = std::collections::HashSet::new();
    providers
        .iter()
        .flat_map(|p| p.option_schema().presets())
        .filter(|p| seen.insert(p.name.as_str()))
        .map(|p| describe(&p.name, p.text))
        .collect()
}

fn describe(name: &str, text: &'static str) -> PresetInfo {
    let tree: serde_json::Value = serde_saphyr::from_str(text).unwrap_or_default();
    let style = tree.as_object().is_some_and(|sections| {
        sections
            .values()
            .any(|s| s.get("format").is_some_and(|f| f.is_object()))
    });
    // The first sentence of the comment at the top of the file.
    let comment: Vec<&str> = text
        .lines()
        .take_while(|l| l.starts_with('#'))
        .map(|l| l.trim_start_matches('#').trim())
        .collect();
    let comment = comment.join(" ");
    PresetInfo {
        name: name.to_string(),
        text,
        description: short(&comment),
        style,
    }
}

/// The first sentence of a preset's comment, cut at its first clause and at 60 characters.
fn short(comment: &str) -> String {
    let first = comment.split_once(". ").map_or(comment, |(a, _)| a);
    let clause = [": ", "; ", " (", ", "]
        .iter()
        .filter_map(|m| first.find(m))
        .min()
        .unwrap_or(first.len());
    let mut text = first[..clause].trim_end_matches('.').to_string();
    if text.chars().count() > 60 {
        let cut: String = text.chars().take(60).collect();
        text = cut
            .rsplit_once(' ')
            .map_or(cut.clone(), |(a, _)| a.to_string());
    }
    text
}

type Scanned = (Vec<Lang>, Vec<(PathBuf, usize)>, bool);

fn scan(dir: &Path, providers: &[Box<dyn LanguageProvider>]) -> Result<Scanned> {
    let seen = std::sync::Arc::new(AtomicUsize::new(0));
    let start = Instant::now();
    let over = {
        let seen = seen.clone();
        move || seen.load(Ordering::Relaxed) >= MAX_FILES || start.elapsed() > MAX_TIME
    };
    let filter = core_workspace::Filter {
        exclude: {
            let over = over.clone();
            std::sync::Arc::new(move |_| over())
        },
        include: std::sync::Arc::new(move |_| {
            seen.fetch_add(1, Ordering::Relaxed);
            true
        }),
    };
    let found = core_workspace::discover_files(&[dir.to_path_buf()], &filter)?;
    let mut langs: Vec<Lang> = Vec::new();
    let mut files = Vec::new();
    for path in found {
        let Some((i, _)) = provider_api::select_provider_index(providers, &path) else {
            continue;
        };
        let name = providers[i].language();
        let at = match langs.iter().position(|l| l.name == name) {
            Some(at) => at,
            None => {
                langs.push(Lang {
                    name: name.to_string(),
                    files: 0,
                    globs: Vec::new(),
                    vscode: crate_ids(providers, name),
                });
                langs.len() - 1
            }
        };
        langs[at].files += 1;
        files.push((path, at));
    }
    for lang in &mut langs {
        lang.globs = globs(providers, &lang.name);
    }
    // Most files first, by name among equals.
    let mut order: Vec<usize> = (0..langs.len()).collect();
    order.sort_by(|&a, &b| (langs[b].files, &langs[a].name).cmp(&(langs[a].files, &langs[b].name)));
    let rank = |i: usize| order.iter().position(|&o| o == i).expect("ranked");
    let files = files.into_iter().map(|(p, i)| (p, rank(i))).collect();
    let mut sorted: Vec<Option<Lang>> = langs.into_iter().map(Some).collect();
    let langs = order
        .iter()
        .map(|&i| sorted[i].take().expect("once"))
        .collect();
    Ok((langs, files, over()))
}

fn crate_ids(providers: &[Box<dyn LanguageProvider>], language: &str) -> Vec<String> {
    providers
        .iter()
        .filter(|p| p.language() == language)
        .flat_map(|p| languages::vscode_languages(p.crate_name()))
        .map(String::from)
        .collect()
}

/// Globs for the files of `language`; a pattern with exceptions cannot be written as one.
fn globs(providers: &[Box<dyn LanguageProvider>], language: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for m in providers
        .iter()
        .filter(|p| p.language() == language)
        .flat_map(|p| p.file_matchers())
    {
        let glob = match &m.pattern {
            FilePattern::Extension(e) => format!("**/*.{e}"),
            FilePattern::FileName(n) => format!("**/{n}"),
            FilePattern::InDirectory { dir, extension } => format!("**/{dir}/*.{extension}"),
            FilePattern::Except { .. } => continue,
        };
        if !out.contains(&glob) {
            out.push(glob);
        }
    }
    out
}

/// Every tool whose config is here and maps to something; the ones that cannot be read are
/// returned as messages.
fn detect() -> (Vec<Found>, Vec<String>) {
    let mut tools = Vec::new();
    let mut problems: Vec<String> = Vec::new();
    for &tool in Tool::value_variants() {
        match migrate(tool) {
            Ok((out, source)) if !out.is_empty() => {
                let name = tool
                    .to_possible_value()
                    .expect("named")
                    .get_name()
                    .to_string();
                tools.push(Found {
                    tool,
                    name,
                    source,
                    out,
                });
            }
            Ok(_) => {}
            Err(e) if e.downcast_ref::<NotFound>().is_some() => {}
            Err(e) => {
                let text = format!("{e:#}");
                if !problems.contains(&text) {
                    problems.push(text);
                }
            }
        }
    }
    (tools, problems)
}
