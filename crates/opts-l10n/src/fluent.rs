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
mod parse;
mod plural;
pub use format::{format, to_ftl_entry, to_source, variables};
pub use parse::{parse, parse_value};
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

#[cfg(test)]
mod tests;
