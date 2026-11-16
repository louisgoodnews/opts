//! Severity and findings: what a check reports and how it prints.

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
