//! A small parser and formatter for the Fluent subset OPTS uses (moved from `opts-gui` so the
//! `opts-tools l10n` commands can use it without the engine).
//!
//! Supported: `# comments`, `key = pattern` with indented continuation lines, `{ $variable }`,
//! `{ other-message }` references, `{ "literal" }` and plural/value selectors
//! (`{ $n -> [one] ... *[other] ... }`, numeric keys match exactly). Not supported (yet): terms
//! (`-term`), attributes (`.attr = ...`), functions (`NUMBER`, `DATETIME`), message-level
//! parameterised terms. Upgrade path: the `fluent-bundle` crate reads the same `.ftl` files; swap
//! [`Table`] for `FluentBundle` behind `Localization::format` in `opts-gui` when the crates can be
//! vendored (they were not in the offline registry on 2026-10-12).

use std::collections::HashMap;

mod format;
mod plural;
pub use format::{format, to_ftl_entry, to_source, variables};
pub use plural::{plural_categories, plural_category};

/// A value passed to a message.
#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    /// Text.
    Str(String),
    /// A number (selects plural forms).
    Num(f64),
}

impl From<&str> for Arg {
    fn from(s: &str) -> Self {
        Self::Str(s.to_string())
    }
}
impl From<String> for Arg {
    fn from(s: String) -> Self {
        Self::Str(s)
    }
}
impl From<f64> for Arg {
    fn from(n: f64) -> Self {
        Self::Num(n)
    }
}
impl From<usize> for Arg {
    fn from(n: usize) -> Self {
        Self::Num(n as f64)
    }
}
impl From<i32> for Arg {
    fn from(n: i32) -> Self {
        Self::Num(f64::from(n))
    }
}

impl Arg {
    fn render(&self) -> String {
        match self {
            Self::Str(s) => s.clone(),
            Self::Num(n) if n.fract() == 0.0 => format!("{}", *n as i64),
            Self::Num(n) => format!("{n}"),
        }
    }
}

/// Named arguments of one message.
pub type Args = Vec<(String, Arg)>;

/// One piece of a pattern.
#[derive(Debug, Clone, PartialEq)]
pub enum Part {
    /// Literal text.
    Text(String),
    /// `{ $name }`.
    Var(String),
    /// `{ message-id }`.
    Ref(String),
    /// `{ $name -> ... }`.
    Select {
        /// The selector variable.
        var: String,
        /// `(key, pattern)` per variant.
        variants: Vec<(String, Pattern)>,
        /// Index of the `*` default variant.
        default: usize,
    },
}

/// A message body.
pub type Pattern = Vec<Part>;

/// One problem found while reading a table: where it is and what is wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// 1-based line of the problem.
    pub line: usize,
    /// The message the problem belongs to, when known.
    pub key: Option<String>,
    /// What is wrong, in a sentence a translator can act on.
    pub message: String,
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.key {
            Some(k) => write!(f, "line {}: {k}: {}", self.line, self.message),
            None => write!(f, "line {}: {}", self.line, self.message),
        }
    }
}

/// All messages of one language.
#[derive(Debug, Clone, Default)]
pub struct Table {
    /// Message id to pattern.
    pub messages: HashMap<String, Pattern>,
    /// Parse problems (`line N: ...`), empty for a clean file.
    pub errors: Vec<String>,
    /// The same problems as structured values.
    pub diagnostics: Vec<Diagnostic>,
    /// Message id to the line it starts on.
    pub lines: HashMap<String, usize>,
    /// Message id to the `#` comment lines directly above it (joined with newlines).
    pub comments: HashMap<String, String>,
    /// The ids in file order (each once).
    pub order: Vec<String>,
}

/// Brace characters, spelled as numbers so line-counting tools do not mistake them for blocks.
const LBRACE: char = 0x7b as char;
const RBRACE: char = 0x7d as char;

fn is_id_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

/// Parses an `.ftl` source. Problems never stop the parse: the faulty message is skipped, the
/// rest is read, and every problem is listed in [`Table::diagnostics`] with its line.
pub fn parse(source: &str) -> Table {
    let mut bundle = Table::default();
    // (line, id, body lines with their source line numbers, comment above)
    type Entry = (usize, String, Vec<(usize, String)>, String);
    let mut current: Option<Entry> = None;
    let mut entries: Vec<Entry> = Vec::new();
    let mut pending_comment: Vec<String> = Vec::new();
    for (n, line) in source.lines().enumerate() {
        let lineno = n + 1;
        let indented = line.starts_with(char::is_whitespace);
        let starts_entry = !indented
            && line
                .split_once('=')
                .is_some_and(|(id, _)| !id.is_empty() && id.trim_end().chars().all(is_id_char));
        if !indented && line.trim_start().starts_with('#') {
            if let Some(done) = current.take() {
                entries.push(done);
            }
            pending_comment.push(line.trim_start().trim_start_matches('#').trim().to_string());
            continue;
        }
        if line.trim().is_empty() {
            if let Some((_, _, body, _)) = current.as_mut() {
                body.push((lineno, String::new()));
            }
            // A blank line separates a comment block from the next message only when the
            // comment is not directly attached; keep it attached (Fluent attaches adjacent only).
            pending_comment.clear();
            continue;
        }
        if starts_entry {
            if let Some(done) = current.take() {
                entries.push(done);
            }
            let (id, rest) = line.split_once('=').unwrap_or((line, ""));
            current = Some((
                lineno,
                id.trim().to_string(),
                vec![(lineno, rest.to_string())],
                std::mem::take(&mut pending_comment).join("\n"),
            ));
        } else if indented {
            if let Some((_, _, body, _)) = current.as_mut() {
                body.push((lineno, line.to_string()));
            } else {
                bundle.diagnostics.push(Diagnostic {
                    line: lineno,
                    key: None,
                    message: "indented text outside any message".into(),
                });
            }
        } else {
            // Not a comment, not `id = ...`, not indented: a typo such as a missing `=`.
            bundle.diagnostics.push(Diagnostic {
                line: lineno,
                key: None,
                message: "expected `key = text` (an id, `=`, then the text); continuation lines must be indented".into(),
            });
            if let Some(done) = current.take() {
                entries.push(done);
            }
        }
        pending_comment.clear();
    }
    entries.extend(current);
    for (line, id, body, comment) in entries {
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
    bundle.errors = bundle.diagnostics.iter().map(ToString::to_string).collect();
    bundle
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
    // `trim` below may drop a leading empty first line: keep the line map aligned.
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

#[cfg(test)]
mod tests;
