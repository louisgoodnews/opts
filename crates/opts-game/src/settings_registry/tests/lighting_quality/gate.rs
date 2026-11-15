//! Lighting quality: the gate is open only in a running, loaded drive.

use super::*;

#[test]
fn the_gate_is_open_only_in_a_running_loaded_drive() {
    use crate::app_state::AppState;
    use crate::in_game::types::{LoadingScreenRoot, SimulationLoadState};
    use opts_bevy::light_quality::GovernorGate;
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::state::app::StatesPlugin)
        .init_state::<AppState>()
        .init_resource::<GovernorGate>()
        .init_resource::<SimulationLoadState>()
        .add_systems(Update, lighting::update_governor_gate);
    let open = |app: &mut App| {
        app.update();
        app.world().resource::<GovernorGate>().open
    };
    assert!(!open(&mut app), "not in a drive");
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::InGame);
    app.update();
    assert!(!open(&mut app), "the drive is still loading");
    app.world_mut()
        .resource_mut::<SimulationLoadState>()
        .is_complete = true;
    assert!(open(&mut app), "loaded and running");
    let overlay = app.world_mut().spawn(LoadingScreenRoot).id();
    assert!(!open(&mut app), "the loading overlay is up");
    app.world_mut().despawn(overlay);
    assert!(open(&mut app));
    app.world_mut().resource_mut::<Time<Virtual>>().pause();
    assert!(!open(&mut app), "paused");
    app.world_mut().resource_mut::<Time<Virtual>>().unpause();
    assert!(open(&mut app));
}
