//! Esc in a drive opens the overlay; Esc inside it resumes.


use super::*;


/// Esc (or Start) in a running drive opens the overlay; during loading Esc cancels the load.
pub fn pause_input(
    mut commands: Commands,
    signals: params::PauseSignals,
    input: params::PauseGate,
    mut pause: ResMut<Pause>,
    mut time: ResMut<Time<Virtual>>,
    mut next: ResMut<NextState<AppState>>,
) {
    let params::PauseSignals {
        mut actions,
        mut requests,
    } = signals;
    let params::PauseGate { gate, keys, load } = input;
    let mut open = requests.read().count() > 0;
    for action in actions.read() {
        match action {
            // Backspace is also "Back" for the UI, but a drive uses it for the vehicle.
            UiAction::Back
                if !gate.closed()
                    && (keys.just_pressed(KeyCode::Escape)
                        || !keys.just_pressed(KeyCode::Backspace)) =>
            {
                if load.is_complete {
                    open = true;
                } else {
                    tracing::info!("Back while loading: cancelling the load");
                    next.set(AppState::MainMenu);
                }
            }
            UiAction::Menu if !gate.closed() && load.is_complete => open = true,
            _ => {}
        }
    }
    if open && load.is_complete {
        open_pause(&mut commands, &mut pause, &mut time);
    }
}

/// Esc inside the overlay resumes.
pub fn pause_cancel(
    mut commands: Commands,
    mut events: MessageReader<ModalEvent>,
    roots: Query<(), With<ModalRoot>>,
    rebuild: Option<Res<opts_bevy::ChronoRebuild>>,
    mut pause: ResMut<Pause>,
    mut time: ResMut<Time<Virtual>>,
) {
    for ModalEvent::CancelRequested(modal) in events.read() {
        if pause.root == Some(*modal) && roots.contains(*modal) {
            environment::resume_drive(&mut commands, &mut pause, &mut time, rebuild.as_deref());
        }
    }
}
