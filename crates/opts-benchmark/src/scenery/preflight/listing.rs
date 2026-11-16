//! The survey: one line per object for `scenery preflight` (no rendering).

use opts_bevy::OmsiInstall;

use super::{run, ObjectEntry, StringPlan};

/// One line of `scenery preflight`: what the CPU-only preflight knows about an object.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SurveyLine {
    /// Relative `.sco` path.
    pub sco: String,
    /// Tier (`referenced`, `attachment`, `unreferenced`).
    pub tier: String,
    /// Context classes.
    pub class: Vec<String>,
    /// Unsupported-feature tags.
    pub unsupported: Vec<String>,
    /// Preflight failure categories.
    pub categories: Vec<String>,
    /// Number of text-texture strings the placement carries.
    pub text_strings: usize,
}

/// Runs the preflight over `entries` (no rendering) and returns one line per object.
#[must_use]
pub fn survey(install: &OmsiInstall, entries: &[ObjectEntry]) -> Vec<SurveyLine> {
    entries
        .iter()
        .map(|e| {
            let p = run(install, e);
            SurveyLine {
                sco: e.sco.clone(),
                tier: e.tier.clone(),
                unsupported: p.unsupported(),
                text_strings: match p.strings {
                    StringPlan::Text(n) => n,
                    _ => 0,
                },
                class: p.classes,
                categories: p.categories,
            }
        })
        .collect()
}
