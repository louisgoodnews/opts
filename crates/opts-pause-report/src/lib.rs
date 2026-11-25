//! The report dialog's state for the pause menu and the other screens that open it.
//!
//! Moved out of `opts-game`'s pause module (ISS-068 stage 1): it needs only the reporting state
//! of `opts-bevy`, so the help and session-summary screens can call it through the old path.

use bevy::prelude::*;
use opts_bevy::reporting::{ReportModalData, ReportModalState};

/// The report dialog's state exists even in headless runs (where the reporting plugin is absent).
pub fn ensure_report_state(app: &mut App) {
    if !app.world().contains_resource::<State<ReportModalState>>() {
        app.init_state::<ReportModalState>();
    }
    app.init_resource::<ReportModalData>();
}
