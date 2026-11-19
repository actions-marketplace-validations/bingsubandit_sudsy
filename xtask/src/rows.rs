//! Row validation: every `format.rules` file is parsed against its language's grammar. An unknown
//! kind, tag, key or shape fails here (the parsers of `core_format` panic on one), and so does a
//! spacing row that an earlier row of its section always matches first.

use anyhow::{Result, bail};
use core_format::rules::RuleFormatter;
use core_syntax::Grammar;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

pub fn check(root: &Path) -> Result<()> {
    let mut errors = Vec::new();
    let mut crates: Vec<_> = std::fs::read_dir(root.join("crates"))?
        .flatten()
        .map(|e| e.path())
        .collect();
    crates.sort();
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    for dir in crates {
        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        let Some(lang) = name.strip_prefix("lang-") else {
            continue;
        };
        let Ok(text) = std::fs::read_to_string(dir.join("format.rules")) else {
            continue;
        };
        let text: &'static str = Box::leak(text.into_boxed_str());
        let grammar = Grammar::parse(&std::fs::read_to_string(
            root.join(format!("grammars/{lang}.grammar")),
        )?)?;
        let parsed = catch_unwind(AssertUnwindSafe(|| {
            let rows = RuleFormatter::parse(&grammar, text, &Default::default());
            rows.calls(&grammar);
            core_format::spacing::shadowed_rows(&grammar, text)
        }));
        match parsed {
            Ok(shadowed) => errors.extend(
                shadowed
                    .into_iter()
                    .map(|m| format!("{name}/format.rules: {m}")),
            ),
            Err(p) => {
                let msg = p
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_default();
                errors.push(format!("{name}/format.rules: {msg}"));
            }
        }
    }
    std::panic::set_hook(hook);
    if errors.is_empty() {
        Ok(())
    } else {
        bail!("rows:\n  {}", errors.join("\n  "))
    }
}
