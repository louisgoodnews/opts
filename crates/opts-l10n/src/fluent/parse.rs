//! Parsing logic for Fluent FTL files.

use super::{Diagnostic, Part, Pattern, Table, LBRACE, RBRACE};

fn is_id_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

type Entry = (usize, String, Vec<(usize, String)>, String);

struct LineScanner<'a> {
    bundle: &'a mut Table,
    current: Option<Entry>,
    entries: Vec<Entry>,
    pending_comment: Vec<String>,
}

impl<'a> LineScanner<'a> {
    fn new(bundle: &'a mut Table) -> Self {
        Self {
            bundle,
            current: None,
            entries: Vec::new(),
            pending_comment: Vec::new(),
        }
    }

    fn scan(mut self, source: &str) -> Vec<Entry> {
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

    fn handle_line(&mut self, line: &str, lineno: usize, starts_entry: bool, indented: bool) {
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

/// Parses an `.ftl` source. Problems never stop the parse: the faulty message is skipped, the
/// rest is read, and every problem is listed in [`Table::diagnostics`] with its line.
pub fn parse(source: &str) -> Table {
    let mut bundle = Table::default();
    let scanner = LineScanner::new(&mut bundle);
    let entries = scanner.scan(source);
    for entry in entries {
        process_entry(entry, &mut bundle);
    }
    bundle.errors = bundle.diagnostics.iter().map(ToString::to_string).collect();
    bundle
}

fn process_entry(entry: Entry, bundle: &mut Table) {
    let (line, id, body, comment) = entry;
    let (text, line_of) = dedent(&body);
    match parse_pattern(&text) {
        Ok(p) => {
            if bundle.messages.insert(id.clone(), p).is_some() {
                bundle.diagnostics.push(Diagnostic {
                    line,
                    key: Some(id.clone()),
                    message: "duplicate key (the later definition wins)".into(),
                });
            } else {
                bundle.order.push(id.clone());
            }
            bundle.lines.insert(id.clone(), line);
            if !comment.is_empty() {
                bundle.comments.insert(id, comment);
            }
        }
        Err(e) => {
            let at = e
                .offset
                .map(|o| text.chars().take(o).filter(|c| *c == '\n').count())
                .and_then(|i| line_of.get(i).copied())
                .unwrap_or(line);
            bundle.diagnostics.push(Diagnostic {
                line: at,
                key: Some(id),
                message: e.message,
            });
        }
    }
}

/// Joins the body lines: the first line keeps its text, continuation lines lose their common
/// indent. Also returns the source line of each output line.
fn dedent(body: &[(usize, String)]) -> (String, Vec<usize>) {
    let first_line = body.first().map_or(1, |(n, _)| *n);
    let first = body.first().map_or("", |(_, s)| s.trim());
    let rest: Vec<&(usize, String)> = body
        .iter()
        .skip(1)
        .filter(|(_, l)| !l.trim().is_empty())
        .collect();
    let indent = rest
        .iter()
        .map(|(_, l)| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);
    let mut out = first.to_string();
    let mut lines = vec![first_line];
    for (n, l) in body.iter().skip(1) {
        if l.trim().is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(l[indent.min(l.len())..].trim_end());
        lines.push(*n);
    }
    let lead = out.len() - out.trim_start().len();
    let dropped = out[..lead].matches('\n').count();
    let lines = lines.split_at(dropped.min(lines.len())).1.to_vec();
    (out.trim().to_string(), lines)
}

/// A pattern problem: what and, when known, the character offset.
struct PatternError {
    message: String,
    offset: Option<usize>,
}

impl From<String> for PatternError {
    fn from(message: String) -> Self {
        Self {
            message,
            offset: None,
        }
    }
}

impl From<&str> for PatternError {
    fn from(message: &str) -> Self {
        message.to_string().into()
    }
}

fn parse_pattern(text: &str) -> Result<Pattern, PatternError> {
    let mut parts = Vec::new();
    let mut buf = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == LBRACE {
            let mut depth = 1;
            let mut j = i + 1;
            while j < chars.len() && depth > 0 {
                match chars[j] {
                    c if c == LBRACE => depth += 1,
                    c if c == RBRACE => depth -= 1,
                    _ => {}
                }
                j += 1;
            }
            if depth != 0 {
                return Err(PatternError {
                    message: "unclosed `{` (every opening brace needs a closing one)".into(),
                    offset: Some(i),
                });
            }
            if !buf.is_empty() {
                push_text(&mut parts, std::mem::take(&mut buf));
            }
            let inner: String = chars[i + 1..j - 1].iter().collect();
            match parse_placeable(inner.trim()).map_err(|mut e| {
                e.offset.get_or_insert(i);
                e
            })? {
                Part::Text(t) => push_text(&mut parts, t),
                other => parts.push(other),
            }
            i = j;
        } else {
            buf.push(chars[i]);
            i += 1;
        }
    }
    if !buf.is_empty() {
        push_text(&mut parts, buf);
    }
    Ok(parts)
}

/// Appends literal text, joining it to a literal before it.
fn push_text(parts: &mut Pattern, text: String) {
    if let Some(Part::Text(last)) = parts.last_mut() {
        last.push_str(&text);
    } else {
        parts.push(Part::Text(text));
    }
}

/// Parses the text of one message value (the right-hand side of `key =`) on its own.
///
/// # Errors
/// A message and the character offset of the problem when known.
pub fn parse_value(text: &str) -> Result<Pattern, (String, Option<usize>)> {
    parse_pattern(text.trim()).map_err(|e| (e.message, e.offset))
}

fn parse_placeable(inner: &str) -> Result<Part, PatternError> {
    if let Some((head, body)) = inner.split_once("->") {
        let var = head
            .trim()
            .strip_prefix('$')
            .ok_or("selector must be a $variable")?;
        return parse_select(var.trim(), body);
    }
    if let Some(var) = inner.strip_prefix('$') {
        return Ok(Part::Var(var.trim().to_string()));
    }
    if inner.len() >= 2 && inner.starts_with('"') && inner.ends_with('"') {
        return Ok(Part::Text(inner[1..inner.len() - 1].to_string()));
    }
    if inner
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic())
        && inner.chars().all(is_id_char)
    {
        return Ok(Part::Ref(inner.to_string()));
    }
    Err(format!("unsupported placeable `{{ {inner} }}` (use `$variable`, a message id or a plural selector)").into())
}

fn parse_select(var: &str, body: &str) -> Result<Part, PatternError> {
    let mut variants: Vec<(String, String)> = Vec::new();
    let mut default = None;
    for line in body.lines() {
        let t = line.trim();
        let (star, rest) = match t.strip_prefix('*') {
            Some(r) => (true, r),
            None => (false, t),
        };
        if let Some(r) = rest.strip_prefix('[') {
            let (key, text) = r.split_once(']').ok_or("`[` without `]`")?;
            if star {
                default = Some(variants.len());
            }
            variants.push((key.trim().to_string(), text.trim().to_string()));
        } else if let Some(last) = variants.last_mut() {
            if !t.is_empty() {
                last.1.push('\n');
                last.1.push_str(t);
            }
        } else if !t.is_empty() {
            return Err("text before the first variant".into());
        }
    }
    let default = default.ok_or("selector needs a `*[default]` variant")?;
    let variants = variants
        .into_iter()
        .map(|(k, v)| parse_pattern(&v).map(|p| (k, p)))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Part::Select {
        var: var.to_string(),
        variants,
        default,
    })
}
