//! Loading pipeline state machine, progressive entity synthesis, and UI synchronization.

use bevy::prelude::*;
use opts_gui::interaction::events::ButtonClicked;
use opts_gui::prelude::*;

use crate::app_state::AppState;
use crate::catalogue_service::CatalogueService;
use crate::in_game::hud::spawn_in_game_hud;
use crate::in_game::omsi_world::{
    begin_world_load, decodes_pending, load_next_tile, load_player_vehicle, prewarm_vehicle_meshes,
    resolve_session_config, spawn_debug_overlay, LoadAssets, SessionRequest, WorldLoadJob,
    WorldStats,
};
use crate::in_game::types::{
    CancelLoadingBtn, InGameSessionInfo, LoadStage, LoadingScreenRoot, ProgressBarFill,
    ProgressPercentText, SimulationLoadState, StageIndicatorText, StartDrivingBtn,
    StatusTickerText,
};
use crate::in_game::vehicle_spawner::{spawn_camera_rig, spawn_free_camera};
use crate::in_game::world::spawn_atmosphere_and_lighting;
use crate::new_game_screen::types::{GameCatalogue, NewGameSetup};
use crate::AppArgs;
use opts_bevy::{CameraMode, CameraState};

/// Advances the loading state machine. Each stage performs real work against the OMSI install:
/// progress reflects tiles actually parsed and spawned, not elapsed time.
#[allow(clippy::too_many_arguments)]
pub fn update_loading_progress(
    mut commands: Commands,
    time: Res<Time>,
    mut state: ResMut<SimulationLoadState>,
    setup: Res<NewGameSetup>,
    mut session: ResMut<InGameSessionInfo>,
    theme: Res<MaterialTheme>,
    app_args: Option<Res<AppArgs>>,
    catalogue: Option<Res<GameCatalogue>>,
    service: Option<Res<CatalogueService>>,
    mut assets: LoadAssets,
    job: Option<ResMut<WorldLoadJob>>,
    mut stats: ResMut<WorldStats>,
    mut camera_state: ResMut<CameraState>,
    loading_screen_query: Query<Entity, With<LoadingScreenRoot>>,
) {
    if state.hud_spawned {
        return;
    }

    // Headless runs (CI, tests) have no meaningful frame time; use a fixed step so the
    // ready-delay elapses. Loading itself is real work and does not depend on this value.
    let is_headless = app_args.as_ref().map(|a| a.0.headless).unwrap_or(true);
    let dt = if is_headless {
        0.35
    } else {
        time.delta_secs().min(0.1)
    };
    state.stage_elapsed += dt;

    match state.stage {
        LoadStage::InitEngine => {
            // First run: the catalogue is still being built; menus and resolution need it.
            if let Some(svc) = &service {
                let empty = catalogue.as_ref().is_none_or(|c| c.maps.is_empty());
                if empty
                    && matches!(
                        svc.status(),
                        crate::catalogue_service::DiscoveryStatus::Running(_)
                    )
                {
                    state.status_message = "Building the OMSI catalogue (first run)...".to_string();
                    return;
                }
            }
            session.map_name = setup.selected_map.clone();
            session.entry_point = setup.selected_entry_point.clone();
            session.vehicle_name = setup.selected_vehicle.clone();
            session.livery_name = setup.selected_livery.clone();
            session.weather_preset = setup.weather_preset.clone();
            session.start_hour = setup.hour;
            session.start_minute = setup.minute;

            let args = app_args.as_ref().map(|a| &a.0);
            let config = resolve_session_config(
                SessionRequest {
                    map_path: args.and_then(|a| a.map_path.as_deref()),
                    omsi_path: args.and_then(|a| a.omsi_path.as_deref()),
                    vehicle_path: args.and_then(|a| a.vehicle_path.as_deref()),
                    selected_map: &setup.selected_map,
                    selected_vehicle: &setup.selected_vehicle,
                    selected_entry_point: &setup.selected_entry_point,
                },
                catalogue.as_deref(),
                service.as_deref(),
            )
            .and_then(|cfg| begin_world_load(cfg).map(|job| (cfg_name(&job), job)));
            *stats = WorldStats {
                overlay_visible: stats.overlay_visible,
                ..default()
            };
            match config {
                Ok((name, job)) => {
                    stats.map_name = name;
                    stats.start_tile = job.start_tile();
                    session.map_name = stats.map_name.clone();
                    commands.insert_resource(job);
                    state.progress = 0.1;
                    state.stage = LoadStage::BuildTerrain;
                    state.status_message = format!("Loading map {}...", stats.map_name);
                }
                Err(e) => {
                    tracing::error!("cannot load OMSI world: {e}");
                    state.status_message = format!("ERROR: {e}");
                    state.progress = 0.7;
                    state.stage = LoadStage::SetupAtmosphere;
                }
            }
        }
        LoadStage::BuildTerrain => {
            if let Some(mut job) = job {
                if let Some(tile) = job.next_tile() {
                    state.status_message = format!("Loading tile {tile:?}...");
                    load_next_tile(
                        &mut job,
                        &mut stats,
                        &mut commands,
                        &mut assets.meshes,
                        &mut assets.materials,
                        &mut assets.images,
                        assets.ground_materials.as_deref_mut(),
                    );
                }
                state.progress = 0.1 + 0.55 * job.progress();
                if job.tiles_done() {
                    state.world_spawned = true;
                    state.stage = LoadStage::SetupAtmosphere;
                    state.status_message = format!(
                        "{} scenery objects, {} splines loaded",
                        stats.objects_loaded, stats.splines_loaded
                    );
                }
            } else {
                state.stage = LoadStage::SetupAtmosphere;
            }
        }
        LoadStage::SetupAtmosphere => {
            if !state.atmosphere_spawned {
                let sky = job.as_ref().map(|j| {
                    super::sky::spawn_sky(
                        &mut commands,
                        &mut assets.meshes,
                        &mut assets.materials,
                        &mut assets.images,
                        j.omsi_root(),
                        &setup,
                    )
                });
                match sky {
                    Some(Ok(())) => {}
                    other => {
                        if let Some(Err(e)) = other {
                            tracing::error!(error = %e, "sky unavailable; using default sun light");
                        }
                        spawn_atmosphere_and_lighting(&mut commands, &setup);
                    }
                }
                state.atmosphere_spawned = true;
            }
            state.progress = 0.7;
            state.stage = LoadStage::SpawnVehicle;
            state.status_message = "Loading vehicle model...".to_string();
        }
        LoadStage::SpawnVehicle => {
            if !state.vehicle_spawned {
                if let Some(mut job) = job {
                    // Scrambled vehicle meshes decode on background threads; the vehicle is
                    // built only once they are all done.
                    if !state.vehicle_prewarmed {
                        state.vehicle_wait = prewarm_vehicle_meshes(
                            &mut job,
                            &mut commands,
                            &mut assets.meshes,
                            &mut assets.materials,
                            &mut assets.images,
                        );
                        state.vehicle_prewarmed = true;
                    }
                    let pending = decodes_pending(&job, &state.vehicle_wait);
                    if pending > 0 {
                        state.status_message =
                            format!("Decoding protected vehicle meshes ({pending} left)...");
                        return;
                    }
                    if let Err(e) = load_player_vehicle(
                        &mut job,
                        &mut stats,
                        &mut commands,
                        &mut assets.meshes,
                        &mut assets.materials,
                        &mut assets.images,
                        &mut assets.bindposes,
                    ) {
                        tracing::error!("cannot load player vehicle: {e}");
                        state.status_message = format!("ERROR: {e}");
                    }
                }
                state.vehicle_spawned = true;
            }
            state.progress = 0.9;
            state.stage = LoadStage::CalibrateControls;
        }
        LoadStage::CalibrateControls => {
            if !state.camera_spawned {
                if let Some(job) = job {
                    if let Some(cam) = job.camera {
                        spawn_camera_rig(&mut commands, &cam);
                        let want_free = app_args
                            .as_ref()
                            .and_then(|a| a.0.camera.as_deref())
                            .is_some_and(|m| m.eq_ignore_ascii_case("free"));
                        let want_ego = app_args
                            .as_ref()
                            .and_then(|a| a.0.camera.as_deref())
                            .is_some_and(|m| m.eq_ignore_ascii_case("ego"));
                        let named = app_args
                            .as_ref()
                            .and_then(|a| a.0.camera.as_deref())
                            .map(str::to_ascii_lowercase);
                        camera_state.mode = if named.as_deref() == Some("cockpit") {
                            camera_state.sub_camera_index = job.camera_std_index;
                            CameraMode::Cockpit
                        } else if named.as_deref() == Some("pax") {
                            CameraMode::Passenger
                        } else if want_ego {
                            CameraMode::Ego
                        } else if want_free {
                            // Developer aid: `OPTS_DEV_CAM_HEIGHT=<m>` starts the free camera
                            // high above the spawn, looking down (overview screenshots).
                            let mut start = cam.start;
                            if let Some(h) = std::env::var("OPTS_DEV_CAM_HEIGHT")
                                .ok()
                                .and_then(|v| v.parse::<f32>().ok())
                            {
                                let focus = cam.start.translation + *cam.start.forward() * h * 0.5;
                                start = Transform::from_translation(
                                    cam.start.translation + Vec3::Y * h,
                                )
                                .looking_at(
                                    Vec3::new(focus.x, cam.start.translation.y, focus.z),
                                    Vec3::Y,
                                );
                                commands.queue(move |w: &mut World| {
                                    let mut q =
                                        w.query_filtered::<&mut Transform, With<Camera3d>>();
                                    for mut t in q.iter_mut(w) {
                                        *t = start;
                                    }
                                });
                            }
                            camera_state.free_roam_spawn = start.translation;
                            let (yaw, pitch, _) = start.rotation.to_euler(EulerRot::YXZ);
                            camera_state.free_roam_yaw = yaw;
                            camera_state.free_roam_pitch = pitch;
                            CameraMode::FreeRoam
                        } else {
                            CameraMode::External
                        };
                    } else if let Some(tf) = job.inspection_camera() {
                        // No vehicle: still give the user a camera to inspect the loaded world.
                        tracing::warn!(
                            "no vehicle loaded; starting in free-roam over the spawn point"
                        );
                        spawn_free_camera(&mut commands, tf);
                        camera_state.mode = CameraMode::FreeRoam;
                        camera_state.free_roam_spawn = tf.translation;
                        let (yaw, pitch, _) = tf.rotation.to_euler(EulerRot::YXZ);
                        camera_state.free_roam_yaw = yaw;
                        camera_state.free_roam_pitch = pitch;
                    }
                    camera_state.previous_mode = camera_state.mode;
                    tracing::info!(mode = ?camera_state.mode, "initial camera mode");
                }
                spawn_debug_overlay(&mut commands);
                state.camera_spawned = true;
            }
            state.progress = 1.0;
            state.stage = LoadStage::Ready;
            state.is_complete = true;
            state.status_message = "Ready".to_string();
        }
        LoadStage::Ready => {
            state.auto_launch_delay -= dt;
            if state.auto_launch_delay <= 0.0 {
                transition_to_active_simulation(
                    &mut commands,
                    &mut state,
                    &session,
                    &theme,
                    &loading_screen_query,
                );
            }
        }
    }
}

fn cfg_name(job: &WorldLoadJob) -> String {
    job.map_name()
}

/// Updates progress bar, percentages, and telemetry ticker text widgets in the loading UI.
#[allow(clippy::type_complexity)]
pub fn update_loading_ui(
    state: Res<SimulationLoadState>,
    mut fill_query: Query<&mut Node, With<ProgressBarFill>>,
    mut percent_query: Query<
        &mut Text,
        (
            With<ProgressPercentText>,
            Without<StatusTickerText>,
            Without<StageIndicatorText>,
        ),
    >,
    mut ticker_query: Query<
        &mut Text,
        (
            With<StatusTickerText>,
            Without<ProgressPercentText>,
            Without<StageIndicatorText>,
        ),
    >,
    mut stage_query: Query<
        &mut Text,
        (
            With<StageIndicatorText>,
            Without<ProgressPercentText>,
            Without<StatusTickerText>,
        ),
    >,
) {
    if !state.is_changed() {
        return;
    }

    let pct = (state.progress * 100.0).clamp(5.0, 100.0);

    for mut style in &mut fill_query {
        style.width = Val::Percent(pct);
    }

    for mut text in &mut percent_query {
        text.0 = format!("{:.0}%", pct);
    }

    for mut text in &mut ticker_query {
        text.0 = format!("> {}", state.status_message);
    }

    for mut text in &mut stage_query {
        let badge = match state.stage {
            LoadStage::InitEngine => "STAGE 1/5: ENGINE INIT",
            LoadStage::BuildTerrain => "STAGE 2/5: GENERATING TERRAIN",
            LoadStage::SetupAtmosphere => "STAGE 3/5: ATMOSPHERE & LIGHTING",
            LoadStage::SpawnVehicle => "STAGE 4/5: SPAWNING BUS FLEET",
            LoadStage::CalibrateControls => "STAGE 5/5: CALIBRATING CONTROLS",
            LoadStage::Ready => "READY: COMMENCING TRANSIT",
        };
        text.0 = badge.to_string();
    }
}

/// Handles interaction with the "Cancel" and "Start Driving Now" buttons on the loading screen.
#[allow(clippy::too_many_arguments)]
pub fn handle_loading_interactions(
    mut commands: Commands,
    mut events: MessageReader<ButtonClicked>,
    cancel_query: Query<&CancelLoadingBtn>,
    start_query: Query<&StartDrivingBtn>,
    mut state: ResMut<SimulationLoadState>,
    session: Res<InGameSessionInfo>,
    theme: Res<MaterialTheme>,
    loading_screen_query: Query<Entity, With<LoadingScreenRoot>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for event in events.read() {
        if cancel_query.contains(event.0) {
            tracing::info!("User cancelled simulation loading; returning to Main Menu");
            next_state.set(AppState::MainMenu);
        } else if start_query.contains(event.0) && state.is_complete {
            tracing::info!("User triggered immediate launch into active simulation");
            transition_to_active_simulation(
                &mut commands,
                &mut state,
                &session,
                &theme,
                &loading_screen_query,
            );
        }
    }
}

/// Transitions from the loading screen to active gameplay: despawns loading overlay and spawns the driving HUD.
fn transition_to_active_simulation(
    commands: &mut Commands,
    state: &mut SimulationLoadState,
    session: &InGameSessionInfo,
    theme: &MaterialTheme,
    loading_screen_query: &Query<Entity, With<LoadingScreenRoot>>,
) {
    // Despawn loading overlay
    for entity in loading_screen_query {
        commands.entity(entity).despawn();
    }

    // Spawn Driving HUD
    if !state.hud_spawned {
        spawn_in_game_hud(commands, theme, session);
        state.hud_spawned = true;
        tracing::info!("Simulation viewport active; driving HUD instantiated");
    }
}
