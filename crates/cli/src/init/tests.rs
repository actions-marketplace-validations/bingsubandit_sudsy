//! The flow of `sudsy init` on scripted answers: the questions, their defaults, cancelling and
//! the files the answers lead to. The terminal itself is covered by `prompt` and by hand.

use super::ask::{Flags, ask};
use super::out::Out;
use super::plan::{Answers, plan};
use super::prompt::{Cancelled, Lines};
use super::world::{Found, World};
use super::{Tool, providers};
use std::fs;
use std::path::Path;

fn tree(files: &[(&str, &str)]) -> tempfile::TempDir {
    let t = tempfile::tempdir().unwrap();
    for (name, text) in files {
        let path = t.path().join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    t
}

fn answers(world: &World, dir: &Path, input: &str) -> anyhow::Result<Answers> {
    let mut p = Lines {
        input: input.as_bytes(),
        out: Vec::new(),
    };
    ask(world, &Flags::default(), dir, &mut p)
}

fn config(world: &World, a: &Answers, dir: &Path) -> String {
    let p = plan(world, a, dir, &dir.join("sudsy.yaml"), "header").unwrap();
    p.files[0].text.clone()
}

#[test]
fn enter_through_every_question_gives_the_defaults() {
    let t = tree(&[("a.py", "x = 1\n"), ("b.json", "{}\n")]);
    let world = World::load(t.path(), &providers(), false).unwrap();
    let a = answers(&world, t.path(), &"\n".repeat(20)).unwrap();
    assert!(a.deselected.is_empty() && a.agents.is_empty() && !a.vscode && !a.ci);
    assert!(a.styles.is_empty() && a.lint_presets.is_empty());
    assert!(config(&world, &a, t.path()).starts_with("# header\n"));
}

#[test]
fn ending_the_input_at_any_question_cancels() {
    let t = tree(&[("a.py", "x = 1\n"), (".vscode/a.json", "{}\n")]);
    let world = World::load(t.path(), &providers(), false).unwrap();
    for answered in 0..3 {
        let err = answers(&world, t.path(), &"\n".repeat(answered)).unwrap_err();
        assert!(err.downcast_ref::<Cancelled>().is_some(), "{answered}");
    }
}

#[test]
fn a_deselected_language_is_excluded_by_its_file_patterns() {
    let t = tree(&[("a.py", ""), ("b.py", ""), ("c.json", "{}\n")]);
    let world = World::load(t.path(), &providers(), false).unwrap();
    // The languages come most files first: python, then json.
    let a = answers(&world, t.path(), "1\n\n\n\n\n").unwrap();
    assert_eq!(a.deselected.iter().collect::<Vec<_>>(), ["json"]);
    assert!(config(&world, &a, t.path()).contains("**/*.json"));
}

#[test]
fn a_pattern_that_would_catch_a_language_that_stays_is_not_written() {
    let t = tree(&[
        ("a.yaml", "a: 1\n"),
        (".github/workflows/ci.yml", "on: push\njobs: {}\n"),
    ]);
    let world = World::load(t.path(), &providers(), false).unwrap();
    let a = Answers {
        deselected: ["yaml".to_string()].into(),
        ..Answers::default()
    };
    let p = plan(&world, &a, t.path(), &t.path().join("sudsy.yaml"), "h").unwrap();
    let text = &p.files[0].text;
    assert!(
        text.contains("**/*.yaml") && !text.contains("**/*.yml"),
        "{text}"
    );
    assert!(
        p.notes.iter().any(|n| n.starts_with("yaml ")),
        "{:?}",
        p.notes
    );
}

fn found(tool: Tool, name: &str, width: i64) -> Found {
    let mut out = Out::new("prettier");
    out.opt_free(&["json"], "indent_size", width);
    Found {
        tool,
        name: name.into(),
        source: format!("{name}.json"),
        out,
    }
}

#[test]
fn two_tools_that_set_one_value_differently_are_asked_which_wins() {
    let t = tree(&[]);
    let mut world = World::load(t.path(), &providers(), false).unwrap();
    world.tools = vec![
        found(Tool::Prettier, "prettier", 2),
        found(Tool::Biome, "biome", 4),
    ];
    let first = answers(&world, t.path(), "\n\n\n").unwrap();
    assert!(config(&world, &first, t.path()).contains("indent_size: 2"));
    let second = answers(&world, t.path(), "\n2\n\n").unwrap();
    assert!(second.take_later.contains("json.format.indent_size"));
    assert!(config(&world, &second, t.path()).contains("indent_size: 4"));
}

#[test]
fn a_preset_named_by_the_user_wins_over_the_tools_settings_for_that_language() {
    let t = tree(&[]);
    let mut world = World::load(t.path(), &providers(), false).unwrap();
    world.tools = vec![found(Tool::Prettier, "prettier", 2)];
    let mut a = Answers {
        tools: vec![0],
        ..Answers::default()
    };
    super::ask::apply_presets(&world, &["json/biome".to_string()], &mut a).unwrap();
    let text = config(&world, &a, t.path());
    assert!(text.contains("preset:json/biome") && !text.contains("indent_size: 2"));
    assert!(super::ask::apply_presets(&world, &["json/nope".to_string()], &mut a).is_err());
}

fn tool_out(name: &'static str, style: Option<&str>, langs: &[&str], width: i64) -> Found {
    let mut out = Out::new(name);
    if let Some(style) = style {
        out.extend(style);
    }
    out.opt_free(langs, "line_width", width);
    Found {
        tool: Tool::Prettier,
        name: name.into(),
        source: format!("{name}.json"),
        out,
    }
}

#[test]
fn keeping_the_tools_reports_the_settings_of_a_tool_that_lost_a_language() {
    let t = tree(&[("a.yaml", "a: 1\n"), ("b.json", "{}\n")]);
    let mut world = World::load(t.path(), &providers(), false).unwrap();
    world.tools = vec![
        tool_out("yamllint", Some("yaml/yamllint"), &[], 0),
        tool_out("prettier", None, &["yaml", "json", "css"], 100),
    ];
    let a = answers(&world, t.path(), &"\n".repeat(8)).unwrap();
    assert_eq!(a.tools, vec![0, 1]);
    assert_eq!(
        a.styles.get("yaml").map(String::as_str),
        Some("yaml/yamllint")
    );
    let p = plan(&world, &a, t.path(), &t.path().join("sudsy.yaml"), "h").unwrap();
    let text = &p.files[0].text;
    assert!(text.contains("preset:yaml/yamllint"), "{text}");
    // The width reaches json; for yaml it is reported, not dropped silently.
    assert!(
        text.contains("json:\n  format:\n    line_width: 100"),
        "{text}"
    );
    assert!(!text.contains("yaml:\n  format"), "{text}");
    let reported: Vec<&String> = p
        .reports
        .iter()
        .flat_map(|(_, r)| &r.not_migrated)
        .collect();
    assert!(
        reported
            .iter()
            .any(|m| m.starts_with("yaml format.line_width of prettier")),
        "{reported:?}"
    );
}

#[test]
fn only_the_languages_found_get_a_section() {
    let t = tree(&[("b.json", "{}\n")]);
    let mut world = World::load(t.path(), &providers(), false).unwrap();
    world.tools = vec![tool_out("prettier", None, &["json", "css", "html"], 100)];
    let a = answers(&world, t.path(), &"\n".repeat(8)).unwrap();
    let text = config(&world, &a, t.path());
    assert!(
        text.contains("json:") && !text.contains("css:") && !text.contains("html:"),
        "{text}"
    );
}
