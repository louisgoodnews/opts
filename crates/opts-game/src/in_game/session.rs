//! Session setup, teardown and developer frame-trace helpers for `InGame`.

use super::*;

/// Initializes simulation resources and spawns the loading screen on entering [`AppState::InGame`].
#[allow(clippy::too_many_arguments)]
pub(crate) fn setup_in_game_session(
    mut commands: Commands,
    mut load_state: ResMut<SimulationLoadState>,
    mut session: ResMut<InGameSessionInfo>,
    mut report: ResMut<load_report::LoadReport>,
    mut environment: MessageWriter<opts_bevy::EnvironmentChange>,
    setup: Res<NewGameSetup>,
    shell: Option<Res<crate::shell::ShellContent>>,
    catalogue: Option<Res<crate::new_game_screen::types::GameCatalogue>>,
    args: Option<Res<crate::AppArgs>>,
) {
    tracing::info!(
        map = ?setup.selected_map,
        vehicle = ?setup.selected_vehicle,
        "Initiating simulation loading sequence"
    );

    // Reset loading state machine
    *load_state = SimulationLoadState::default();
    report.reset();

    // Populate session config
    session.map_name = setup.selected_map.clone();
    session.entry_point = setup.selected_entry_point.clone();
    // The vehicle line is decided here, once, from the selection that came with the load request,
    // so the loading screen shows it from its first frame and it never changes (M-0076).
    let cli_vehicle = args.as_ref().and_then(|a| a.0.vehicle_path.as_deref());
    let cli_livery = args.as_ref().and_then(|a| a.0.livery.clone());
    let default_catalogue = crate::new_game_screen::types::GameCatalogue::default();
    let catalogue = catalogue.as_deref().unwrap_or(&default_catalogue);
    (
        session.vehicle_name,
        session.vehicle_none,
        session.livery_name,
    ) = match cli_vehicle {
        Some(path) => (
            path.file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
            false,
            cli_livery.unwrap_or_default(),
        ),
        None if setup.no_vehicle => (String::new(), true, String::new()),
        None => (
            crate::new_game_screen::texts::vehicle_name(catalogue, &setup),
            false,
            cli_livery
                .unwrap_or_else(|| crate::new_game_screen::texts::livery_name(catalogue, &setup)),
        ),
    };
    session.weather_preset = setup.weather_preset.clone();
    session.start_hour = setup.hour;
    session.start_minute = setup.minute;
    session.sim_elapsed_seconds = 0.0;
    // The session environment starts at the planned date, time and weather; the map is loaded
    // for that date (a restored save sets its clock when it is back, `save::sections`).
    environment.write(opts_bevy::EnvironmentChange::reset(
        crate::new_game_screen::planner::setup_date(&setup),
        f64::from(setup.hour * 3600 + setup.minute * 60),
        setup.weather_preset.clone(),
    ));

    // Spawn Material Design 3 loading screen overlay
    spawn_loading_screen_ui(
        &mut commands,
        &shell.as_deref().copied().unwrap_or_default(),
    );
}

/// Despawns all entities associated with the in-game session on exiting [`AppState::InGame`].
pub(super) fn cleanup_in_game_session(
    mut commands: Commands,
    query: Query<Entity, With<InGameEntity>>,
    mut load_state: ResMut<SimulationLoadState>,
) {
    tracing::info!("Tearing down 3D in-game simulation session and restoring menus");

    for entity in &query {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<omsi_world::WorldLoadJob>();
    commands.remove_resource::<sky::Sky>();

    *load_state = SimulationLoadState::default();
}

/// Streaming profile (M-0089): `OPTS_STREAM_TRACE=<file>` appends one JSON line per frame with the
/// frame period and the `opts_logging::stream_trace` spans recorded since the previous frame.
#[cfg(feature = "stream-trace")]
pub(super) fn stream_trace_frame(
    mut file: Local<Option<std::fs::File>>,
    mut last: Local<Option<std::time::Instant>>,
    mut frame: Local<u64>,
) {
    use std::io::Write;
    let Some(path) = std::env::var_os("OPTS_STREAM_TRACE") else {
        return;
    };
    let now = std::time::Instant::now();
    let dt = last
        .replace(now)
        .map_or(0.0, |p| (now - p).as_secs_f64() * 1e3);
    if file.is_none() {
        *file = std::fs::File::create(path).ok();
    }
    *frame += 1;
    let spans = opts_logging::stream_trace::drain();
    if let Some(f) = file.as_mut() {
        let _ = writeln!(
            f,
            "{}",
            opts_logging::stream_trace::frame_line(*frame, dt, &spans)
        );
    }
}

/// Developer aid: `OPTS_DBG_FRAMES=<file>` appends one CSV line per frame (real time, frame time,
/// player position, speed, fixed steps so far) to measure how smooth the motion is.
pub(super) fn debug_frame_log(
    time: Res<Time<Real>>,
    fixed: Res<Time<Fixed>>,
    player: Query<(&Transform, &opts_bevy::VehicleController), With<opts_bevy::PlayerVehicle>>,
    mut file: Local<Option<std::fs::File>>,
    mut steps: Local<u64>,
    mut last_fixed: Local<f64>,
) {
    use std::io::Write;
    let Some(path) = std::env::var_os("OPTS_DBG_FRAMES") else {
        return;
    };
    let Ok((tf, c)) = player.single() else { return };
    if file.is_none() {
        *file = std::fs::File::create(path).ok();
    }
    let elapsed = fixed.elapsed_secs_f64();
    if elapsed != *last_fixed {
        *steps += ((elapsed - *last_fixed) / fixed.timestep().as_secs_f64()).round() as u64;
        *last_fixed = elapsed;
    }
    if let Some(f) = file.as_mut() {
        let _ = writeln!(
            f,
            "{:.5},{:.5},{:.4},{:.4},{:.3},{}",
            time.elapsed_secs_f64(),
            time.delta_secs_f64(),
            tf.translation.x,
            tf.translation.z,
            c.sim.chassis.forward_speed_m_s(),
            *steps
        );
    }
}
