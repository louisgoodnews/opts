//! The `P` key: pauses and resumes the simulation (virtual time), not while a modal is open.

use bevy::prelude::*;

/// `P` pauses and resumes the simulation (virtual time; the free camera and the HUD keep
/// running). Ignored while the F1 report modal is open, whose text field takes the key.
pub(super) fn toggle_pause(
    keyboard: Res<ButtonInput<KeyCode>>,
    menu: Option<Res<opts_bevy::reporting::MenuModalActive>>,
    report: Option<Res<State<opts_bevy::reporting::ReportModalState>>>,
    mut time: ResMut<Time<Virtual>>,
) {
    if report.is_some_and(|r| *r.get() != opts_bevy::reporting::ReportModalState::Inactive)
        || menu.is_some_and(|m| m.0)
        || !keyboard.just_pressed(KeyCode::KeyP)
    {
        return;
    }
    if time.is_paused() {
        time.unpause();
        tracing::info!("simulation resumed [P]");
    } else {
        time.pause();
        tracing::info!("simulation paused [P]");
    }
}

