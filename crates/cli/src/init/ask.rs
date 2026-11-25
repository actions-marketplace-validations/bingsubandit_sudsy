//! The questions of `sudsy init`. Each has a default taken from the repository and is skipped
//! when a flag already answers it; the answers are the struct of [`plan`](super::plan).

use super::plan::{Answers, config};
use super::prompt::{Item, Prompter};
use super::world::World;
use super::{Tool, agents};
use anyhow::{Result, bail};
use std::collections::BTreeMap;

/// What the command line already answers.
#[derive(Default)]
pub struct Flags {
    pub languages: Option<Vec<String>>,
    pub presets: Vec<String>,
    pub from: Vec<Tool>,
    pub agents: bool,
    pub vscode: bool,
    pub ci: bool,
}

pub fn ask(
    world: &World,
    flags: &Flags,
    dir: &std::path::Path,
    p: &mut dyn Prompter,
) -> Result<Answers> {
    let mut a = Answers::default();
    languages(world, flags, p, &mut a)?;
    style(world, flags, p, &mut a)?;
    lint(world, flags, p, &mut a)?;
    agents(world, flags, dir, p, &mut a)?;
    editor(world, flags, p, &mut a)?;
    Ok(a)
}

fn languages(world: &World, flags: &Flags, p: &mut dyn Prompter, a: &mut Answers) -> Result<()> {
    let chosen: Vec<String> = match &flags.languages {
        Some(names) => names.clone(),
        None if world.langs.is_empty() => {
            p.say("No files of a known language here; the config starts without a style.");
            return Ok(());
        }
        None => {
            if world.capped {
                p.say("The scan stopped early; the counts are partial.");
            }
            let items: Vec<Item> = world
                .langs
                .iter()
                .map(|l| {
                    Item::new(
                        &l.name,
                        format!("{} file{}", l.files, if l.files == 1 { "" } else { "s" }),
                    )
                })
                .collect();
            let checked = vec![true; items.len()];
            let at = p.multi("Languages to set up", &items, &checked)?;
            at.into_iter()
                .map(|i| world.langs[i].name.clone())
                .collect()
        }
    };
    a.deselected = world
        .langs
        .iter()
        .map(|l| l.name.clone())
        .filter(|n| !chosen.contains(n))
        .collect();
    Ok(())
}

/// The style preset a detected tool's config maps to for `language`.
fn tool_style(world: &World, language: &str) -> Option<String> {
    world
        .tools
        .iter()
        .find_map(|t| world.own_style(&t.out, language))
        .map(String::from)
}

/// The styles the detected tools' own presets stand for, where no preset was named: their
/// settings are kept as they are.
pub fn keep_tool_styles(world: &World, a: &mut Answers) {
    for lang in world.tools.iter().flat_map(|t| t.out.languages()) {
        if let Some(style) = tool_style(world, &lang) {
            a.styles.entry(lang).or_insert(style);
        }
    }
}

/// Presets named by the user: a style replaces the language's, a rule-only one is added.
pub fn apply_presets(world: &World, names: &[String], a: &mut Answers) -> Result<()> {
    for name in names {
        let Some(info) = world.preset(name) else {
            let known: Vec<&str> = world.presets.iter().map(|p| p.name.as_str()).collect();
            bail!(
                "unknown preset `{name}`; known presets: {}",
                known.join(", ")
            );
        };
        if info.style {
            a.styles
                .insert(super::out::language_of(name).into(), name.clone());
        } else {
            a.lint_presets.push(name.clone());
        }
    }
    Ok(())
}

type Groups<'a> = BTreeMap<Vec<&'a str>, Vec<&'a str>>;

/// The languages that offer style presets, grouped by the presets they offer.
fn groups<'a>(world: &'a World, a: &Answers) -> Groups<'a> {
    let mut groups: Groups = BTreeMap::new();
    for lang in world
        .langs
        .iter()
        .filter(|l| !a.deselected.contains(&l.name))
    {
        let short: Vec<&str> = world
            .presets_of(&lang.name, true)
            .map(|p| p.name.split_once('/').map_or(p.name.as_str(), |(_, s)| s))
            .collect();
        if !short.is_empty() {
            groups.entry(short).or_default().push(&lang.name);
        }
    }
    groups
}

/// The style of every language and the tools whose settings are taken over: one question when
/// one answer fits, a question per group of languages only when the user wants to choose.
fn style(world: &World, flags: &Flags, p: &mut dyn Prompter, a: &mut Answers) -> Result<()> {
    for problem in &world.problems {
        p.say(&format!("Cannot migrate: {problem}"));
    }
    if !flags.presets.is_empty() || !flags.from.is_empty() {
        apply_presets(world, &flags.presets, a)?;
        keep_tool_styles(world, a);
        a.tools = flags
            .from
            .iter()
            .filter_map(|t| world.tools.iter().position(|f| f.tool == *t))
            .collect();
        return settle(world, p, a);
    }
    let usable: Vec<usize> = (0..world.tools.len())
        .filter(|&i| !world.covered(&world.tools[i].out, &a.deselected).is_empty())
        .collect();
    let groups = groups(world, a);
    if usable.is_empty() && groups.is_empty() {
        return Ok(());
    }
    // 0: keep the tools, 1: Sudsy's defaults, 2: choose per language.
    let mut ways = Vec::new();
    let mut items = Vec::new();
    if !usable.is_empty() {
        let hint: Vec<String> = usable
            .iter()
            .map(|&i| {
                let t = &world.tools[i];
                let (styled, plain): (Vec<String>, Vec<String>) = world
                    .covered(&t.out, &a.deselected)
                    .into_iter()
                    .partition(|l| world.own_style(&t.out, l).is_some());
                let mut parts = vec![];
                if !styled.is_empty() {
                    parts.push(styled.join(", "));
                }
                if !plain.is_empty() {
                    parts.push(format!(
                        "{} already in its style by default",
                        plain.join(", ")
                    ));
                }
                format!("{}: {}", t.name, parts.join("; "))
            })
            .collect();
        ways.push(0);
        items.push(Item::new("Keep the tools found", hint.join("; ")));
    }
    ways.push(1);
    items.push(Item::new("Sudsy's defaults", "no preset"));
    if !groups.is_empty() {
        ways.push(2);
        items.push(Item::new("Choose per language", ""));
    }
    match ways[p.select("Style", &items, 0)?] {
        0 => keep(world, &usable, p, a)?,
        2 => {
            per_language(world, groups, p, a)?;
            migrate_choice(world, p, a)?;
        }
        _ => {}
    }
    settle(world, p, a)
}

/// Takes over the tools' settings and their styles; where two tools stand for different styles
/// of one language the user picks (the other tool's settings for it are reported as not migrated).
fn keep(world: &World, usable: &[usize], p: &mut dyn Prompter, a: &mut Answers) -> Result<()> {
    a.tools = usable.to_vec();
    let mut by_lang: BTreeMap<String, Vec<(usize, String)>> = BTreeMap::new();
    for &i in usable {
        let out = &world.tools[i].out;
        for lang in world.covered(out, &a.deselected) {
            let style = world.own_style(out, &lang).map(String::from);
            if let Some(style) = style {
                by_lang.entry(lang).or_default().push((i, style));
            }
        }
    }
    for (lang, mut options) in by_lang {
        options.dedup_by(|x, y| x.1 == y.1);
        let at = if options.len() == 1 {
            0
        } else {
            let items: Vec<Item> = options
                .iter()
                .map(|(i, style)| {
                    Item::new(
                        style.split_once('/').map_or(style.as_str(), |(_, s)| s),
                        format!("from {}", world.tools[*i].name),
                    )
                })
                .collect();
            p.select(&format!("Style for {lang}"), &items, 0)?
        };
        a.styles.insert(lang, options.swap_remove(at).1);
    }
    Ok(())
}

fn per_language(
    world: &World,
    groups: Groups,
    p: &mut dyn Prompter,
    a: &mut Answers,
) -> Result<()> {
    for (short, langs) in groups {
        let first = langs[0];
        let own = tool_style(world, first);
        let mut items: Vec<Item> = short
            .iter()
            .map(|s| {
                let name = format!("{first}/{s}");
                let info = world.preset(&name).expect("listed");
                Item::new(*s, info.description.clone())
            })
            .collect();
        items.push(Item::new("Sudsy defaults", "no preset"));
        let default = own
            .and_then(|o| short.iter().position(|s| o.ends_with(&format!("/{s}"))))
            .unwrap_or(items.len() - 1);
        let at = p.select(&format!("Style for {}", langs.join(", ")), &items, default)?;
        if let Some(s) = short.get(at) {
            for lang in langs {
                a.styles.insert(lang.into(), format!("{lang}/{s}"));
            }
        }
    }
    Ok(())
}

/// With a style chosen per language: which tools' settings to take over.
fn migrate_choice(world: &World, p: &mut dyn Prompter, a: &mut Answers) -> Result<()> {
    if world.tools.is_empty() {
        return Ok(());
    }
    let items: Vec<Item> = world
        .tools
        .iter()
        .map(|t| {
            let langs = world.covered(&t.out, &a.deselected);
            Item::new(&t.name, format!("{} ({})", t.source, langs.join(", ")))
        })
        .collect();
    let checked: Vec<bool> = world
        .tools
        .iter()
        .map(|t| {
            world
                .covered(&t.out, &a.deselected)
                .iter()
                .any(|l| a.styles.get(l).map(String::as_str) == world.own_style(&t.out, l))
        })
        .collect();
    a.tools = p.multi("Migrate the settings of these tools", &items, &checked)?;
    Ok(())
}

/// Two tools that set one value differently: the user decides, nothing is dropped silently.
fn settle(world: &World, p: &mut dyn Prompter, a: &mut Answers) -> Result<()> {
    let (_, conflicts, _, _) = config(world, a);
    for c in conflicts {
        let items = [
            Item::new(&c.kept, "from the first config"),
            Item::new(&c.other, "from the second config"),
        ];
        let at = p.select(&format!("`{}` is set twice", c.path), &items, 0)?;
        if at == 1 {
            a.take_later.insert(c.path);
        }
    }
    Ok(())
}

fn lint(world: &World, flags: &Flags, p: &mut dyn Prompter, a: &mut Answers) -> Result<()> {
    if !flags.presets.is_empty() {
        return Ok(());
    }
    let mut items = Vec::new();
    let mut checked = Vec::new();
    for lang in world
        .langs
        .iter()
        .filter(|l| !a.deselected.contains(&l.name))
    {
        for (n, preset) in world.presets_of(&lang.name, false).enumerate() {
            items.push((
                preset.name.clone(),
                Item::new(&preset.name, &preset.description),
            ));
            checked.push(n == 0);
        }
    }
    if items.is_empty() {
        return Ok(());
    }
    let levels = [
        Item::new("Likely bugs only", "what Sudsy reports without a preset"),
        Item::new(
            "Full rule sets",
            "the rules of the tools' presets, chosen next",
        ),
    ];
    if p.select("Lint level", &levels, 0)? == 1 {
        let shown: Vec<Item> = items
            .iter()
            .map(|(_, i)| Item::new(&i.label, &i.hint))
            .collect();
        for at in p.multi("Rule sets to add", &shown, &checked)? {
            a.lint_presets.push(items[at].0.clone());
        }
    }
    Ok(())
}

fn agents(
    world: &World,
    flags: &Flags,
    dir: &std::path::Path,
    p: &mut dyn Prompter,
    a: &mut Answers,
) -> Result<()> {
    if flags.agents {
        a.agents.push(agents::default_file(dir));
        return Ok(());
    }
    let items: Vec<Item> = agents::FILES
        .iter()
        .map(|f| {
            Item::new(
                *f,
                if dir.join(f).exists() {
                    "exists"
                } else {
                    "new file"
                },
            )
        })
        .collect();
    let at = p.multi(
        "Tell AI agents how to run Sudsy in",
        &items,
        &world.agent_files,
    )?;
    a.agents = at.into_iter().map(|i| agents::FILES[i]).collect();
    Ok(())
}

fn editor(world: &World, flags: &Flags, p: &mut dyn Prompter, a: &mut Answers) -> Result<()> {
    if flags.vscode {
        (a.vscode, a.format_on_save) = (true, true);
    } else if world.vscode_dir && p.confirm("Recommend the VS Code extension?", false)? {
        a.vscode = true;
        a.format_on_save = p.confirm("Format on save with Sudsy?", false)?;
    }
    a.ci = flags.ci
        || (world.github
            && p.confirm("Add a GitHub Actions job that runs `sudsy check`?", false)?);
    Ok(())
}
