//! The scan status chip of *Settings → Content*: is a manual scan recommended?
//!
//! The decision is the pure function [`assess`]; everything else gathers its inputs, shows the
//! result (icon + text + reason, never colour alone) and refreshes it every few seconds.
//!
//! Rules (documented in `docs/src/roadmap/phase-9/M-0086.md`), first match wins:
//!
//! | Level | Reason | When |
//! | :--- | :--- | :--- |
//! | red | folder missing | no OMSI folder is set or it cannot be found |
//! | red | failed | the last scan failed |
//! | red | never scanned | the catalogue has no completed scan |
//! | red | empty | the last scan found no entries |
//! | yellow | running | a scan is running now |
//! | yellow | folder changed | the OMSI folder or one of its top-level folders was modified after the last scan |
//! | yellow | older | the last scan is older than the automatic scan interval |
//! | green | recent | none of the above |

use bevy::prelude::*;
use opts_gui::interaction::tooltip::Tooltip;
use opts_gui::l10n::{L10nArgs, L10nKey, Localization};
use opts_gui::prelude::*;

use crate::catalogue_service::CatalogueService;
use crate::content_facts::ContentFactsStore;
pub use crate::content_facts::RootState;
use crate::settings::GameSettings;
use crate::shell::icons;

mod chip;
mod chip_facts;
mod chip_parts;
mod labels;
mod refresh;
mod verdict;

use chip_parts::{ChipParts, ReasonQuery};

pub use chip::{ScanChip, ScanChipIcon, ScanChipLabel, ScanChipReason, spawn_chip};
pub use labels::{level_key, look, reason_key};
pub use refresh::refresh_scan_chip;
pub use verdict::{assess, folder_changed_since};

/// How urgent a scan is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanLevel {
    /// Green: nothing to do.
    Good,
    /// Yellow: a scan is recommended.
    Recommended,
    /// Red: the catalogue cannot be trusted or does not exist.
    Needed,
}

/// Why the level is what it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanReason {
    /// No OMSI folder, or it is gone.
    FolderMissing,
    /// The last scan failed.
    Failed,
    /// No completed scan yet.
    NeverScanned,
    /// The last scan found nothing.
    Empty,
    /// A scan is running.
    Running,
    /// The OMSI folder changed after the last scan.
    FolderChanged,
    /// The last scan is older than the automatic interval.
    Older,
    /// Scanned recently, nothing changed.
    Recent,
}

/// The verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanStatus {
    /// Colour and urgency.
    pub level: ScanLevel,
    /// The reason shown to the user.
    pub reason: ScanReason,
}

/// Everything [`assess`] looks at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanInputs {
    /// Unix time now.
    pub now: i64,
    /// Unix time of the last completed scan.
    pub last_scan: Option<i64>,
    /// Entries the catalogue holds.
    pub entities: i64,
    /// The automatic scan interval in days (the setting).
    pub interval_days: u32,
    /// The OMSI folder.
    pub root: RootState,
    /// Modification times in the OMSI folder are newer than the last scan.
    pub folder_changed: bool,
    /// The last scan failed.
    pub failed: bool,
    /// A scan is running.
    pub running: bool,
}

#[cfg(test)]
mod tests;
