//! Opening the report dialog from code (the pause menu, Help, the session summary).

use bevy::prelude::*;

use super::state::{ReportModalData, ReportModalState};

/// Opens the report dialog from code (the pause menu, the Help page, a banner), like F1 once did: the screenshot is taken first, then the dialog opens.
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
