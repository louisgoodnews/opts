//! Reports of the languages the interface can be shown in: the summary of each one and its
//! pack-check findings.

use std::path::PathBuf;

use opts_l10n::check::Issue;
use opts_l10n::pack::{Direction, Pack};

/// What the localisation keeps of a pack after loading it.
#[derive(Debug, Clone)]
pub(super) struct LoadedPack {
    pub(super) pack: Pack,
}

/// A language the interface can be shown in (a built-in one or an installed pack).
#[derive(Debug, Clone, PartialEq)]
pub struct LanguageInfo {
    /// The tag (`de`, `fr`, `pt-BR`).
    pub code: String,
    /// English name.
    pub name: String,
    /// Name in the language itself.
    pub native_name: String,
    /// Writing direction.
    pub direction: Direction,
    /// `true` for `en` and `de` (shipped), `false` for a pack.
    pub builtin: bool,
    /// `true` when a pack changes or extends this language.
    pub from_pack: bool,
    /// Authors named by the pack (built-in languages: the project).
    pub authors: Vec<String>,
    /// Version of the pack.
    pub version: String,
    /// Number of English messages.
    pub total: usize,
    /// Messages really translated.
    pub translated: usize,
    /// Messages present but untouched copies of the English text.
    pub copied: usize,
    /// Messages that fall back to English (missing).
    pub missing: usize,
    /// Findings of the pack check.
    pub errors: usize,
    /// Warnings of the pack check.
    pub warnings: usize,
    /// The pack was made against an older English text.
    pub outdated: bool,
    /// Scripts used that the bundled fonts do not cover.
    pub missing_fonts: Vec<String>,
    /// The folder of the pack.
    pub dir: Option<PathBuf>,
    /// `false` for a right-to-left language (not supported yet).
    pub selectable: bool,
}

impl LanguageInfo {
    /// Percentage of English messages that are translated, 0 to 100.
    #[must_use]
    pub fn percent(&self) -> f64 {
        if self.total == 0 {
            100.0
        } else {
            self.translated as f64 * 100.0 / self.total as f64
        }
    }
}

/// The report of one language: completeness, the keys that fall back to English, and every
/// finding of the pack check.
#[derive(Debug, Clone)]
pub struct LanguageReport {
    /// The language.
    pub info: LanguageInfo,
    /// Keys that show English.
    pub missing_keys: Vec<String>,
    /// Keys present but identical to English prose.
    pub copied_keys: Vec<String>,
    /// Findings, `file:line` relative to the pack folder.
    pub issues: Vec<Issue>,
}
