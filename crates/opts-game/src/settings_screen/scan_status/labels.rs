//! The look and the message keys of a verdict: colours, icon, level word and reason.

use opts_gui::prelude::*;

use crate::shell::icons;

use super::{ScanLevel, ScanReason};

/// Colours and icon of a level: the status container roles, with the icon and the word beside
/// them (colour is never the only signal).
#[must_use]
pub fn look(level: ScanLevel) -> (ColorRole, ColorRole, &'static str) {
    match level {
        ScanLevel::Good => (
            ColorRole::SuccessContainer,
            ColorRole::OnSuccessContainer,
            icons::CHECK_CIRCLE,
        ),
        ScanLevel::Recommended => (
            ColorRole::WarningContainer,
            ColorRole::OnWarningContainer,
            icons::WARNING,
        ),
        ScanLevel::Needed => (
            ColorRole::ErrorContainer,
            ColorRole::OnErrorContainer,
            icons::ERROR,
        ),
    }
}

/// Message key of the level word.
#[must_use]
pub fn level_key(level: ScanLevel) -> &'static str {
    match level {
        ScanLevel::Good => "scan-level-good",
        ScanLevel::Recommended => "scan-level-recommended",
        ScanLevel::Needed => "scan-level-needed",
    }
}

/// Message key of the reason and what to do.
#[must_use]
pub fn reason_key(reason: ScanReason) -> &'static str {
    match reason {
        ScanReason::FolderMissing => "scan-why-folder-missing",
        ScanReason::Failed => "scan-why-failed",
        ScanReason::NeverScanned => "scan-why-never",
        ScanReason::Empty => "scan-why-empty",
        ScanReason::Running => "scan-why-running",
        ScanReason::FolderChanged => "scan-why-changed",
        ScanReason::Older => "scan-why-older",
        ScanReason::Recent => "scan-why-recent",
    }
}
