//! The questions of `sudsy init` as a trait, so the flow holds no terminal code: arrow keys on a
//! terminal (`dialoguer`), numbered lines on a dumb terminal or scripted input, and the defaults
//! for `--yes`.

use anyhow::Result;
use std::io::{BufRead, Write};

/// An answer was given up: Esc, Ctrl-C, or the input ended.
#[derive(Debug)]
pub struct Cancelled;

impl std::fmt::Display for Cancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("cancelled")
    }
}

impl std::error::Error for Cancelled {}

/// One choice: a label and a short description.
pub struct Item {
    pub label: String,
    pub hint: String,
}

impl Item {
    pub fn new(label: impl Into<String>, hint: impl Into<String>) -> Self {
        Item {
            label: label.into(),
            hint: hint.into(),
        }
    }

    /// `label  hint`, cut to `width` columns (the hint goes first).
    fn text(&self, width: usize) -> String {
        let full = if self.hint.is_empty() {
            self.label.clone()
        } else {
            format!("{}  {}", self.label, self.hint)
        };
        console::truncate_str(&full, width.max(12), "...").into_owned()
    }
}

pub trait Prompter {
    fn select(&mut self, question: &str, items: &[Item], default: usize) -> Result<usize>;
    /// The indexes chosen; `checked` marks the ones chosen at the start.
    fn multi(&mut self, question: &str, items: &[Item], checked: &[bool]) -> Result<Vec<usize>>;
    fn confirm(&mut self, question: &str, default: bool) -> Result<bool>;
    /// A line of text between the questions.
    fn say(&mut self, text: &str);
}

/// Takes every default (`--yes`).
pub struct Defaults;

impl Prompter for Defaults {
    fn select(&mut self, _: &str, _: &[Item], default: usize) -> Result<usize> {
        Ok(default)
    }
    fn multi(&mut self, _: &str, _: &[Item], checked: &[bool]) -> Result<Vec<usize>> {
        Ok((0..checked.len()).filter(|&i| checked[i]).collect())
    }
    fn confirm(&mut self, _: &str, default: bool) -> Result<bool> {
        Ok(default)
    }
    fn say(&mut self, text: &str) {
        eprintln!("{text}");
    }
}

/// Arrow keys, space and Enter on a terminal; Esc or Ctrl-C cancel.
pub struct Terminal {
    term: console::Term,
    theme: dialoguer::theme::ColorfulTheme,
}

impl Terminal {
    pub fn new() -> Self {
        // `console` raises SIGINT itself on Ctrl-C: give the cursor back and say nothing was written.
        let set = ctrlc::set_handler(|| {
            let term = console::Term::stderr();
            let _ = term.show_cursor();
            let _ = term.write_line("\ncancelled; nothing written");
            std::process::exit(130);
        });
        HANDLER.store(set.is_ok(), std::sync::atomic::Ordering::Relaxed);
        Terminal {
            term: console::Term::stderr(),
            theme: dialoguer::theme::ColorfulTheme::default(),
        }
    }

    /// The line that stays after a question: its text and the names chosen.
    fn result(&self, question: &str, answer: &str) {
        let width = usize::from(self.term.size().1).saturating_sub(4);
        let line = format!("{} {question} · {answer}", console::style("✔").green());
        let _ = self
            .term
            .write_line(&console::truncate_str(&line, width, "…"));
    }

    /// Columns an item may take: the terminal width minus the marker the library draws.
    fn width(&self) -> usize {
        usize::from(self.term.size().1).saturating_sub(6)
    }
}

/// The Ctrl-C handler is installed: it prints the cancel line and exits.
static HANDLER: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Esc gives `None`, Ctrl-C an `Interrupted` error: both cancel.
fn done<T>(r: dialoguer::Result<Option<T>>) -> Result<T> {
    match r {
        Ok(Some(v)) => Ok(v),
        Ok(None) => Err(Cancelled.into()),
        Err(dialoguer::Error::IO(e)) if e.kind() == std::io::ErrorKind::Interrupted => {
            // The handler is exiting the process; do not race it with a second message.
            while HANDLER.load(std::sync::atomic::Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
            Err(Cancelled.into())
        }
        Err(e) => Err(e.into()),
    }
}

impl Prompter for Terminal {
    fn select(&mut self, question: &str, items: &[Item], default: usize) -> Result<usize> {
        let w = self.width();
        let texts: Vec<String> = items.iter().map(|i| i.text(w)).collect();
        let at = done(
            dialoguer::Select::with_theme(&self.theme)
                .with_prompt(question)
                .items(&texts)
                .default(default)
                .report(false)
                .interact_on_opt(&self.term),
        )?;
        self.result(question, &items[at].label);
        Ok(at)
    }

    fn multi(&mut self, question: &str, items: &[Item], checked: &[bool]) -> Result<Vec<usize>> {
        let w = self.width();
        let texts: Vec<(String, bool)> = items
            .iter()
            .zip(checked)
            .map(|(i, c)| (i.text(w), *c))
            .collect();
        let at = done(
            dialoguer::MultiSelect::with_theme(&self.theme)
                .with_prompt(format!("{question} (space marks, Enter accepts)"))
                .items_checked(texts)
                .report(false)
                .interact_on_opt(&self.term),
        )?;
        let names: Vec<&str> = at.iter().map(|&i| items[i].label.as_str()).collect();
        self.result(
            question,
            &if names.is_empty() {
                "none".into()
            } else {
                names.join(", ")
            },
        );
        Ok(at)
    }

    fn confirm(&mut self, question: &str, default: bool) -> Result<bool> {
        let yes = done(
            dialoguer::Confirm::with_theme(&self.theme)
                .with_prompt(question)
                .default(default)
                .report(false)
                .interact_on_opt(&self.term),
        )?;
        self.result(question, if yes { "yes" } else { "no" });
        Ok(yes)
    }

    fn say(&mut self, text: &str) {
        let _ = self.term.write_line(text);
    }
}

/// Numbered lines read from `input` and written to `out`: a dumb terminal, or scripted input.
/// The end of the input cancels.
pub struct Lines<R, W> {
    pub input: R,
    pub out: W,
}

impl<R: BufRead, W: Write> Lines<R, W> {
    fn line(&mut self, prompt: &str) -> Result<String> {
        write!(self.out, "{prompt}")?;
        self.out.flush()?;
        let mut line = String::new();
        if self.input.read_line(&mut line)? == 0 {
            return Err(Cancelled.into());
        }
        Ok(line.trim().to_string())
    }

    fn list(&mut self, question: &str, items: &[Item], marks: &[bool]) -> Result<()> {
        writeln!(self.out, "{question}")?;
        for (i, item) in items.iter().enumerate() {
            let mark = if marks[i] { "*" } else { " " };
            writeln!(self.out, " {mark}{:>2}) {}", i + 1, item.text(70))?;
        }
        Ok(())
    }
}

impl<R: BufRead, W: Write> Prompter for Lines<R, W> {
    fn select(&mut self, question: &str, items: &[Item], default: usize) -> Result<usize> {
        let marks: Vec<bool> = (0..items.len()).map(|i| i == default).collect();
        self.list(question, items, &marks)?;
        loop {
            let answer = self.line(&format!("number, Enter for {}: ", default + 1))?;
            if answer.is_empty() {
                return Ok(default);
            }
            match answer.parse::<usize>() {
                Ok(n) if (1..=items.len()).contains(&n) => return Ok(n - 1),
                _ => writeln!(self.out, "not a choice: {answer}")?,
            }
        }
    }

    fn multi(&mut self, question: &str, items: &[Item], checked: &[bool]) -> Result<Vec<usize>> {
        self.list(question, items, checked)?;
        let keep: Vec<usize> = (0..items.len()).filter(|&i| checked[i]).collect();
        loop {
            let answer =
                self.line("numbers separated by commas, `none`, Enter keeps the marked: ")?;
            if answer.is_empty() {
                return Ok(keep);
            }
            if answer == "none" {
                return Ok(Vec::new());
            }
            let picked: Vec<usize> = answer
                .split(',')
                .filter_map(|n| n.trim().parse::<usize>().ok())
                .filter(|n| (1..=items.len()).contains(n))
                .map(|n| n - 1)
                .collect();
            if picked.len() == answer.split(',').count() {
                return Ok(picked);
            }
            writeln!(self.out, "not a list of choices: {answer}")?;
        }
    }

    fn confirm(&mut self, question: &str, default: bool) -> Result<bool> {
        let hint = if default { "Y/n" } else { "y/N" };
        loop {
            let answer = self.line(&format!("{question} [{hint}] "))?;
            match answer.to_ascii_lowercase().as_str() {
                "" => return Ok(default),
                "y" | "yes" => return Ok(true),
                "n" | "no" => return Ok(false),
                _ => writeln!(self.out, "answer y or n")?,
            }
        }
    }

    fn say(&mut self, text: &str) {
        let _ = writeln!(self.out, "{text}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(input: &str) -> Lines<&[u8], Vec<u8>> {
        Lines {
            input: input.as_bytes(),
            out: Vec::new(),
        }
    }

    fn items() -> Vec<Item> {
        ["a", "b", "c"].map(|l| Item::new(l, "")).into()
    }

    #[test]
    fn enter_takes_the_default_and_a_number_picks() {
        assert_eq!(lines("\n").select("q", &items(), 1).unwrap(), 1);
        assert_eq!(lines("3\n").select("q", &items(), 1).unwrap(), 2);
        assert_eq!(lines("9\n1\n").select("q", &items(), 1).unwrap(), 0);
    }

    #[test]
    fn multi_keeps_the_marked_unless_a_list_or_none_is_given() {
        let checked = [true, false, true];
        assert_eq!(lines("\n").multi("q", &items(), &checked).unwrap(), [0, 2]);
        assert_eq!(lines("2\n").multi("q", &items(), &checked).unwrap(), [1]);
        assert!(
            lines("none\n")
                .multi("q", &items(), &checked)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            lines("x\n1, 2\n").multi("q", &items(), &checked).unwrap(),
            [0, 1]
        );
    }

    #[test]
    fn the_end_of_the_input_cancels() {
        let err = lines("").confirm("ok?", true).unwrap_err();
        assert!(err.downcast_ref::<Cancelled>().is_some());
        assert!(lines("n\n").confirm("ok?", true).is_ok_and(|v| !v));
    }

    #[test]
    fn long_items_are_cut_to_the_width() {
        let item = Item::new("label", "x".repeat(200));
        assert!(item.text(40).chars().count() <= 40);
    }
}
