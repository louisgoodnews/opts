//! F1 detection.

use bevy::prelude::*;

use super::state::{ReportModalData, ReportModalState};

/// The key that starts a report.
pub const REPORT_KEY: KeyCode = KeyCode::F1;

/// Starts a report when F1 is pressed and none is in progress. Works in every application state
/// (menus, loading, simulation); the modal opens once the screenshot is ready.
pub fn trigger_report_system(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<ReportModalState>>,
    time: Res<Time<Virtual>>,
    mut data: ResMut<ReportModalData>,
    mut next: ResMut<NextState<ReportModalState>>,
) {
    if keys.just_pressed(REPORT_KEY) && *state.get() == ReportModalState::Inactive {
        data.reset_for_new_report(!time.is_paused());
        tracing::debug!(was_running = data.was_running_before, "report requested");
        next.set(ReportModalState::Capturing);
    }
}

/// Opens the report dialog from code (the pause menu, the Help page, a banner), exactly as F1
/// did: the screenshot is taken first, then the dialog opens.
pub fn request_report(
    time: &Time<Virtual>,
    data: &mut ReportModalData,
    state: &State<ReportModalState>,
    next: &mut NextState<ReportModalState>,
) {
    if *state.get() == ReportModalState::Inactive {
        data.reset_for_new_report(!time.is_paused());
        next.set(ReportModalState::Capturing);
    }
}
