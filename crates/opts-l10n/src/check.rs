//! Checking a translation against the English source: the rules behind `opts-tools l10n check`
//! and the language list of the game.
//!
//! Errors make a message unusable: it is **skipped** and the English text shows instead. Warnings
//! are reported and the message is kept.

use std::collections::{BTreeSet, HashMap};

use super::fluent::{self, Part, Pattern, Table};

mod stats;
pub use stats::{scripts_beyond_fonts, stats, Stats};

/// How bad a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Reported, message kept.
    Warning,
    /// Message (or pack) skipped.
    Error,
}

impl Severity {
    /// `error` or `warning`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

/// One finding: where it is, what is wrong and what to do about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    /// Error or warning.
    pub severity: Severity,
    /// A stable identifier (`syntax`, `unknown-key`, `variable-mismatch`, `empty`, ...), listed in
    /// `docs/src/architecture/localisation-packs.md`.
    pub code: &'static str,
    /// The file, relative to the pack folder (or the name the caller gave).
    pub file: String,
    /// 1-based line when known.
    pub line: Option<usize>,
    /// The message key concerned.
    pub key: Option<String>,
    /// What is wrong and how to fix it.
    pub message: String,
}

impl Issue {
    /// A new issue.
    #[must_use]
    pub fn new(
        severity: Severity,
        code: &'static str,
        file: &str,
        line: Option<usize>,
        key: Option<&str>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            code,
            file: file.to_string(),
            line,
            key: key.map(str::to_string),
            message: message.into(),
        }
    }

    /// The JSON form (`severity`, `code`, `file`, `line`, `key`, `message`).
    #[must_use]
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "severity": self.severity.name(),
            "code": self.code,
            "file": self.file,
            "line": self.line,
            "key": self.key,
            "message": self.message,
        })
    }
}

impl std::fmt::Display for Issue {
    /// `file:line: error[code]: message (key)`, the shape compilers use, so editors can jump to it.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.line {
            Some(l) => write!(f, "{}:{l}: ", self.file)?,
            None => write!(f, "{}: ", self.file)?,
        }
        write!(
            f,
            "{}[{}]: {}",
            self.severity.name(),
            self.code,
            self.message
        )?;
        if let Some(k) = &self.key {
            write!(f, " (key `{k}`)")?;
        }
        Ok(())
    }
}

/// The English source every translation is checked against.
#[derive(Debug, Clone, Default)]
pub struct Reference {
    /// All English messages (comments and file order included).
    pub table: Table,
    /// Message key to the name of the source file it comes from.
    pub file_of: HashMap<String, String>,
    vars: HashMap<String, BTreeSet<String>>,
}

impl Reference {
    /// A reference from named `.ftl` sources `(file name, text)`; later files may not redefine a
    /// key of an earlier one (the later definition wins).
    #[must_use]
    pub fn from_sources(sources: &[(&str, &str)]) -> Self {
        let mut table = Table::default();
        let mut file_of = HashMap::new();
        for (name, text) in sources {
            let t = fluent::parse(text);
            for key in &t.order {
                file_of.insert(key.clone(), (*name).to_string());
                if !table.messages.contains_key(key) {
                    table.order.push(key.clone());
                }
            }
            table.messages.extend(t.messages);
            table.lines.extend(t.lines);
            table.comments.extend(t.comments);
            table.diagnostics.extend(t.diagnostics);
        }
        table.errors = table.diagnostics.iter().map(ToString::to_string).collect();
        Self::from_table(table, file_of)
    }

    /// A reference over an already merged table.
    #[must_use]
    pub fn from_table(table: Table, file_of: HashMap<String, String>) -> Self {
        let vars = table
            .messages
            .iter()
            .map(|(k, p)| (k.clone(), fluent::variables(p)))
            .collect();
        Self {
            table,
            file_of,
            vars,
        }
    }

    /// Number of messages.
    #[must_use]
    pub fn len(&self) -> usize {
        self.table.messages.len()
    }

    /// `true` without messages.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.table.messages.is_empty()
    }

    /// The keys in file order.
    #[must_use]
    pub fn keys(&self) -> &[String] {
        &self.table.order
    }

    /// The variables and references of the English message `key`.
    #[must_use]
    pub fn variables_of(&self, key: &str) -> Option<&BTreeSet<String>> {
        self.vars.get(key)
    }

    /// A short hash of all English keys and texts: a pack records it as `base_version`, so a later
    /// check can tell that the source has moved on.
    #[must_use]
    pub fn revision(&self) -> String {
        let mut keys: Vec<&String> = self.table.messages.keys().collect();
        keys.sort();
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for k in keys {
            let line = format!("{k}={}\n", fluent::to_source(&self.table.messages[k]));
            for b in line.bytes() {
                h ^= u64::from(b);
                h = h.wrapping_mul(0x0100_0000_01b3);
            }
        }
        format!("{:08x}", h >> 32 ^ (h & 0xffff_ffff))
    }
}

/// A table after validation: only the messages that passed, and the findings.
#[derive(Debug, Clone, Default)]
pub struct Checked {
    /// The accepted messages (`messages`, `lines`, `order`, `comments` are filled).
    pub table: Table,
    /// All findings.
    pub issues: Vec<Issue>,
}

mod rules;
pub use rules::*;

#[cfg(test)]
mod tests;
