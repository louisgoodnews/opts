//! In-game transit driving simulation lifecycle, loading orchestration, and telemetry HUD.

pub mod attachments;
pub mod camera;
pub mod hot_reload;
pub mod hud;
pub mod loading_pipeline;
pub mod loading_ui;
pub mod omsi_world;
pub mod screenshot;
pub mod sky;
pub mod types;
pub mod vehicle_spawner;
pub mod world;

use bevy::prelude::*;
use opts_gui::prelude::*;

use crate::app_state::AppState;
use crate::in_game::camera::{deactivate_menu_camera, reactivate_menu_camera};
use crate::in_game::hud::{handle_camera_shortcuts, handle_hud_interactions, update_in_game_hud};
use crate::in_game::loading_pipeline::{
    handle_loading_interactions, update_loading_progress, update_loading_ui,
};
use crate::in_game::loading_ui::spawn_loading_screen_ui;
use crate::new_game_screen::types::NewGameSetup;

mod vehicle_menu;

pub use camera::MenuCamera;
pub use types::{
    CancelLoadingBtn, InGameCamera, InGameEntity, InGameHudRoot, InGameSessionInfo, LoadStage,
    LoadingScreenRoot, PauseMenuBtn, ProgressBarFill, ProgressPercentText, SimulationLoadState,
    SpeedometerText, StageIndicatorText, StartDrivingBtn, StatusTickerText, TachometerText,
};
pub use vehicle_menu::{MenuCommands, VehicleMenu, VehicleMenuPlugin};

/// Bevy plugin managing the active transit simulation lifecycle, loading sequence, and driving HUD.
#[derive(Default)]
pub struct InGamePlugin;

impl Plugin for InGamePlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>()
            .init_resource::<omsi_world::GroundIndex>()
            .init_resource::<omsi_world::CollisionIndex>()
            .init_resource::<omsi_world::RailNetwork>()
            .init_resource::<omsi_world::BikeNetwork>()
            .init_resource::<omsi_world::BikeSettings>()
            .init_resource::<omsi_world::BikeAiPool>()
            .init_resource::<omsi_world::RailSettings>()
            .init_resource::<omsi_world::RailAiPool>()
            .init_resource::<omsi_world::PedestrianNetwork>()
            .init_resource::<omsi_world::PedestrianSettings>()
            .init_resource::<omsi_world::PedestrianPool>()
            .init_resource::<omsi_world::TrafficState>()
            .init_resource::<omsi_world::TrafficSettings>()
            .init_resource::<omsi_world::TrafficPool>()
            .init_resource::<SimulationLoadState>()
            .init_resource::<InGameSessionInfo>()
            .init_resource::<opts_bevy::CameraState>()
            .init_resource::<opts_bevy::CameraAvailability>()
            .insert_resource(omsi_world::WorldStats {
                overlay_visible: true,
                ..default()
            });

        app.add_plugins(VehicleMenuPlugin).add_systems(
            Update,
            omsi_world::handle_vehicle_requests.run_if(in_state(AppState::InGame)),
        );

        // Lifecycle hooks for entering/exiting AppState::InGame
        app.add_systems(
            FixedUpdate,
            omsi_world::rail_player_step
                .in_set(opts_bevy::RailSet)
                .after(opts_bevy::vehicle_physics_step_system)
                .run_if(in_state(AppState::InGame)),
        );
        app.add_systems(
            FixedUpdate,
            omsi_world::player_collisions
                .after(opts_bevy::vehicle_physics_step_system)
                .after(opts_bevy::RailSet)
                .before(opts_bevy::articulation_system)
                .run_if(in_state(AppState::InGame)),
        );
        app.add_systems(
            Update,
            omsi_world::attach_traffic_colliders.run_if(in_state(AppState::InGame)),
        );
        app.add_systems(
            OnEnter(AppState::InGame),
            (
                deactivate_menu_camera,
                setup_in_game_session,
                omsi_world::configure_pedestrians,
                omsi_world::configure_rail,
                |mut i: ResMut<omsi_world::CollisionIndex>| {
                    *i = omsi_world::CollisionIndex::default()
                },
                omsi_world::configure_traffic,
            ),
        )
        .add_systems(
            OnExit(AppState::InGame),
            (
                omsi_world::log_final_position,
                |mut t: ResMut<Time<Virtual>>| t.unpause(),
                cleanup_in_game_session,
                reactivate_menu_camera,
            )
                .chain(),
        );

        // Active simulation frame updates
        app.add_systems(
            Update,
            (
                update_loading_progress,
                update_loading_ui,
                update_in_game_hud,
                omsi_world::stream_world_tiles,
                omsi_world::track_position,
                toggle_pause,
                debug_frame_log
                    .after(omsi_world::follow_ground)
                    .after(omsi_world::rail_player_pose),
                omsi_world::load_rail_variants,
                omsi_world::spawn_ai_trains,
                omsi_world::move_ai_trains,
                omsi_world::rail_player_pose.after(opts_bevy::vehicle_transform_sync_system),
                omsi_world::follow_ground.after(opts_bevy::vehicle_transform_sync_system),
                (
                    omsi_world::load_variants,
                    omsi_world::spawn_pedestrians,
                    omsi_world::move_pedestrians,
                )
                    .chain(),
                (
                    omsi_world::update_traffic_graph,
                    omsi_world::load_traffic_variants,
                    omsi_world::spawn_traffic,
                    omsi_world::move_traffic,
                    omsi_world::update_ai_variables,
                )
                    .chain(),
                sky::update_sky.run_if(resource_exists::<sky::Sky>),
                screenshot::capture_screenshot_when_ready,
                handle_camera_shortcuts,
            )
                .run_if(in_state(AppState::InGame)),
        );

        // Input-driven systems stand still while the report modal is open.
        app.add_systems(
            Update,
            (
                handle_loading_interactions,
                omsi_world::update_debug_overlay,
                handle_hud_interactions,
                escape_to_menu_in_game,
            )
                .run_if(in_state(AppState::InGame))
                .run_if(opts_bevy::reporting::not_modal_active),
        );
    }
}

/// Initializes simulation resources and spawns the loading screen on entering [`AppState::InGame`].
fn setup_in_game_session(
    mut commands: Commands,
    mut load_state: ResMut<SimulationLoadState>,
    mut session: ResMut<InGameSessionInfo>,
    setup: Res<NewGameSetup>,
    theme: Res<MaterialTheme>,
) {
    tracing::info!(
        map = ?setup.selected_map,
        vehicle = ?setup.selected_vehicle,
        "Initiating simulation loading sequence"
    );

    // Reset loading state machine
    *load_state = SimulationLoadState::default();

    // Populate session config
    session.map_name = setup.selected_map.clone();
    session.entry_point = setup.selected_entry_point.clone();
    session.vehicle_name = setup.selected_vehicle.clone();
    session.livery_name = setup.selected_livery.clone();
    session.weather_preset = setup.weather_preset.clone();
    session.start_hour = setup.hour;
    session.start_minute = setup.minute;
    session.sim_elapsed_seconds = 0.0;

    // Spawn Material Design 3 loading screen overlay
    spawn_loading_screen_ui(&mut commands, &theme, &setup);
}

/// Despawns all entities associated with the in-game session on exiting [`AppState::InGame`].
fn cleanup_in_game_session(
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

/// Escape key listener returning immediately to the Main Menu from in-game simulation.
fn escape_to_menu_in_game(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        tracing::info!("User pressed [Escape] - exiting in-game session to Main Menu");
        next_state.set(AppState::MainMenu);
    }
}

/// `P` pauses and resumes the simulation (virtual time; the free camera and the HUD keep
/// running). Ignored while the F1 report modal is open, whose text field takes the key.
fn toggle_pause(
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

/// Developer aid: `OPTS_DBG_FRAMES=<file>` appends one CSV line per frame (real time, frame time,
/// player position, speed, fixed steps so far) to measure how smooth the motion is.
fn debug_frame_log(
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
