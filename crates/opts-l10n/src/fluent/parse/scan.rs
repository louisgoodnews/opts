//! The line scanner of the Fluent parser.

use super::*;

pub(super) fn is_id_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

pub(super) struct LineScanner<'a> {
    bundle: &'a mut Table,
    current: Option<Entry>,
    entries: Vec<Entry>,
    pending_comment: Vec<String>,
}

impl<'a> LineScanner<'a> {
    pub(super) fn new(bundle: &'a mut Table) -> Self {
        Self {
            bundle,
            current: None,
            entries: Vec::new(),
            pending_comment: Vec::new(),
        }
    }

    pub(super) fn scan(mut self, source: &str) -> Vec<Entry> {
        for (n, line) in source.lines().enumerate() {
            let lineno = n + 1;
            let indented = line.starts_with(char::is_whitespace);
            let starts_entry = !indented
                && line
                    .split_once('=')
                    .is_some_and(|(id, _)| !id.is_empty() && id.trim_end().chars().all(is_id_char));
            if !indented && line.trim_start().starts_with('#') {
                if let Some(done) = self.current.take() {
                    self.entries.push(done);
                }
                self.pending_comment
                    .push(line.trim_start().trim_start_matches('#').trim().to_string());
                continue;
            }
            if line.trim().is_empty() {
                if let Some((_, _, body, _)) = self.current.as_mut() {
                    body.push((lineno, String::new()));
                }
                self.pending_comment.clear();
                continue;
            }
            self.handle_line(line, lineno, starts_entry, indented);
        }
        self.entries.extend(self.current);
        self.entries
    }

    pub(super) fn handle_line(&mut self, line: &str, lineno: usize, starts_entry: bool, indented: bool) {
        if starts_entry {
            if let Some(done) = self.current.take() {
                self.entries.push(done);
            }
            let (id, rest) = line.split_once('=').unwrap_or((line, ""));
            self.current = Some((
                lineno,
                id.trim().to_string(),
                vec![(lineno, rest.to_string())],
                std::mem::take(&mut self.pending_comment).join("\n"),
            ));
        } else if indented {
            if let Some((_, _, body, _)) = self.current.as_mut() {
                body.push((lineno, line.to_string()));
            } else {
                self.bundle.diagnostics.push(Diagnostic {
                    line: lineno,
                    key: None,
                    message: "indented text outside any message".into(),
                });
            }
        } else {
            self.bundle.diagnostics.push(Diagnostic {
                line: lineno,
                key: None,
                message: "expected `key = text` (an id, `=`, then the text); continuation lines must be indented".into(),
            });
            if let Some(done) = self.current.take() {
                self.entries.push(done);
            }
        }
        self.pending_comment.clear();
    }
}
