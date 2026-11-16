//! Interactive mode: prompt, completion, history and multi-line JSON.

use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::history::FileHistory;
use rustyline::{Context, Editor, Helper, Highlighter, Hinter, Validator};

use crate::client::Client;
use crate::commands::completions;
use crate::output::OutputMode;
use crate::run::execute;

mod help;
#[cfg(test)]
mod tests;
mod words;

use help::show_help;
pub use words::{is_incomplete, split_words};

#[derive(Helper, Highlighter, Hinter, Validator)]
struct CommandHelper;

impl Completer for CommandHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let before = &line[..pos];
        if before.contains(char::is_whitespace) {
            return Ok((pos, Vec::new()));
        }
        let candidates = completions(before)
            .map(|m| Pair {
                display: m.to_string(),
                replacement: m.to_string(),
            })
            .collect();
        Ok((0, candidates))
    }
}

fn history_path() -> Option<std::path::PathBuf> {
    let dirs = directories::ProjectDirs::from("org", "opts", "opts")?;
    let dir = dirs.data_dir();
    std::fs::create_dir_all(dir).ok()?;
    Some(dir.join("cli_history"))
}

/// Runs the interactive loop until `exit` or end of input; returns the exit code.
pub fn run_repl(mut client: Client, mode: OutputMode) -> u8 {
    let Ok(mut editor) = Editor::<CommandHelper, FileHistory>::new() else {
        errln!("cannot start the line editor");
        return 1;
    };
    editor.set_helper(Some(CommandHelper));
    let history = history_path();
    if let Some(path) = &history {
        let _ = editor.load_history(path);
    }
    loop {
        let mut line = match editor.readline("opts> ") {
            Ok(l) => l,
            Err(ReadlineError::Interrupted) => continue,
            Err(ReadlineError::Eof) => break,
            Err(e) => {
                errln!("input error: {e}");
                break;
            }
        };
        while is_incomplete(&line) {
            match editor.readline("  ... ") {
                Ok(more) => {
                    line.push('\n');
                    line.push_str(&more);
                }
                Err(_) => break,
            }
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let _ = editor.add_history_entry(trimmed);
        let words = split_words(trimmed);
        match words.first().map(String::as_str) {
            Some("exit" | "quit") => break,
            Some("?") => show_help(&words[1..]),
            Some("list") => show_help(&[]),
            _ => {
                execute(&mut client, mode, &words);
            }
        }
    }
    if let Some(path) = &history {
        let _ = editor.save_history(path);
    }
    0
}
