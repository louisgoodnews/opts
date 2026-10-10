//! Interactive mode: prompt, completion, history and multi-line JSON.

use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::history::FileHistory;
use rustyline::{Context, Editor, Helper, Highlighter, Hinter, Validator};

use crate::client::Client;
use crate::commands::{completions, find, COMMANDS};
use crate::output::OutputMode;
use crate::run::{execute, out};

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

/// Splits a line into words, honouring `'single'` and `"double"` quotes and `\` escapes.
#[must_use]
pub fn split_words(line: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    let mut started = false;
    let mut chars = line.chars();
    while let Some(ch) = chars.next() {
        match (quote, ch) {
            (Some(q), c) if c == q => quote = None,
            (Some('"') | None, '\\') => {
                if let Some(n) = chars.next() {
                    cur.push(n);
                    started = true;
                }
            }
            (None, '\'' | '"') => {
                quote = Some(ch);
                started = true;
            }
            (None, c) if c.is_whitespace() => {
                if started {
                    words.push(std::mem::take(&mut cur));
                    started = false;
                }
            }
            (_, c) => {
                cur.push(c);
                started = true;
            }
        }
    }
    if started {
        words.push(cur);
    }
    words
}

/// Whether braces/brackets/quotes are still open (the REPL then asks for another line).
#[must_use]
pub fn is_incomplete(text: &str) -> bool {
    let (mut depth, mut in_str, mut esc) = (0i32, false, false);
    for ch in text.chars() {
        if in_str {
            match (esc, ch) {
                (true, _) => esc = false,
                (false, '\\') => esc = true,
                (false, '"') => in_str = false,
                _ => {}
            }
        } else {
            match ch {
                '"' => in_str = true,
                '{' | '[' => depth += 1,
                '}' | ']' => depth -= 1,
                _ => {}
            }
        }
    }
    depth > 0 || in_str
}

fn history_path() -> Option<std::path::PathBuf> {
    let dirs = directories::ProjectDirs::from("org", "opts", "opts")?;
    let dir = dirs.data_dir();
    std::fs::create_dir_all(dir).ok()?;
    Some(dir.join("cli_history"))
}

fn show_help(topic: &[String]) {
    match topic.first().and_then(|t| find(t)) {
        Some(c) => out(&format!(
            "{}\n  {}\n  parameters: {}",
            c.method,
            c.summary,
            if c.params.is_empty() { "-" } else { c.params }
        )),
        None => {
            out("commands (Tab completes, `? <command>` describes one):");
            for c in COMMANDS {
                out(&format!("  {:<22} {}", c.method, c.summary));
            }
            out("exit with `exit` or Ctrl-D");
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_split_like_a_shell() {
        assert_eq!(
            split_words(r#"input.key W --duration 2000"#),
            ["input.key", "W", "--duration", "2000"]
        );
        assert_eq!(
            split_words(r#"report "hello  world" 'a b'"#),
            ["report", "hello  world", "a b"]
        );
        assert_eq!(split_words(r#"x "" y"#), ["x", "", "y"]);
        assert_eq!(split_words(r"a\ b"), ["a b"]);
    }

    #[test]
    fn unbalanced_json_is_incomplete() {
        assert!(is_incomplete(r#"entity.spawn --json {"asset":"a""#));
        assert!(is_incomplete("batch [ "));
        assert!(!is_incomplete(r#"report "text with { brace""#));
        assert!(!is_incomplete(r#"x --json {"a":[1,2]}"#));
    }
}
