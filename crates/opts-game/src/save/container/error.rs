//! The error of a save: why a file could not be read, written or found, and its short kind.

/// Why a save could not be read, written or found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveError {
    /// The file system said no.
    Io(String),
    /// The file is not valid JSON, or not shaped like a save.
    Corrupt(String),
    /// Valid JSON that is not an OPTS save (wrong or missing `magic`).
    NotASave,
    /// Written by a newer build.
    NewerFormat {
        /// The file's `format_version`.
        found: u32,
        /// The newest version this build reads.
        supported: u32,
    },
    /// The file or a section is bigger than the limit.
    TooLarge(String),
    /// No save with this id.
    NotFound(String),
    /// The name or id is not acceptable.
    InvalidName(String),
    /// A save with this name exists already (its id).
    Exists(String),
    /// There is no running session to save.
    NoSession(String),
    /// A section's capture function failed.
    Capture {
        /// Section id.
        section: String,
        /// What went wrong.
        message: String,
    },
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(m) => write!(f, "file error: {m}"),
            Self::Corrupt(m) => write!(f, "the save file is damaged: {m}"),
            Self::NotASave => write!(f, "the file is not an OPTS save"),
            Self::NewerFormat { found, supported } => write!(
                f,
                "the save was written by a newer version (format {found}, this build reads up to {supported})"
            ),
            Self::TooLarge(m) => write!(f, "too large: {m}"),
            Self::NotFound(id) => write!(f, "no save `{id}`"),
            Self::InvalidName(m) => write!(f, "invalid name: {m}"),
            Self::Exists(id) => write!(f, "a save called like this exists already (`{id}`)"),
            Self::NoSession(m) => write!(f, "{m}"),
            Self::Capture { section, message } => {
                write!(f, "section `{section}` could not be captured: {message}")
            }
        }
    }
}

impl std::error::Error for SaveError {}

impl SaveError {
    /// A stable word for logs, the control protocol and the UI's message choice.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Io(_) => "io",
            Self::Corrupt(_) => "corrupt",
            Self::NotASave => "not_a_save",
            Self::NewerFormat { .. } => "newer_format",
            Self::TooLarge(_) => "too_large",
            Self::NotFound(_) => "not_found",
            Self::InvalidName(_) => "invalid_name",
            Self::Exists(_) => "exists",
            Self::NoSession(_) => "no_session",
            Self::Capture { .. } => "capture",
        }
    }
}
