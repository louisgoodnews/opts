//! Pausing and resuming simulation time around the modal.

use bevy::prelude::*;

use super::state::{ModalActive, ReportModalData};

/// Pauses `Time<Virtual>` (only if it was running) and engages input gating.
/// `Time<Real>` is never touched, so input handling and the modal keep working.
pub fn on_report_start(
    mut time: ResMut<Time<Virtual>>,
    mut data: ResMut<ReportModalData>,
    mut active: ResMut<ModalActive>,
) {
    active.0 = true;
    data.paused_by_report = false;
    if data.was_running_before && !time.is_paused() {
        time.pause();
        data.paused_by_report = true;
    }
}

/// Releases gating and resumes time if this report paused it.
pub fn on_report_end(
    mut time: ResMut<Time<Virtual>>,
    mut data: ResMut<ReportModalData>,
    mut active: ResMut<ModalActive>,
) {
    active.0 = false;
    if data.paused_by_report {
        time.unpause();
        data.paused_by_report = false;
    }
}
