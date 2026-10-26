//! Loading pipeline state machine: each stage does real work against the OMSI installation and
//! reports it in plain numbers; everything that goes wrong is recorded in the
//! [`LoadReport`] and never overwritten.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use opts_gui::interaction::Activated;
use opts_gui::prelude::*;

use crate::app_state::AppState;
use crate::catalogue_service::{CatalogueService, DiscoveryStatus};
use crate::in_game::hud::spawn_in_game_hud;
use crate::in_game::load_report::{
    problems_from_counts, AcceptLoadProblems, LoadOutcome, LoadProblem, LoadReport, WorldCounts,
};
use crate::in_game::loading_ui::{
    ChooseAnotherBtn, ContinueBtn, DetailsBtn, LoadingBackBtn, RetryBtn,
};
use crate::in_game::omsi_world::{
    advance_tile_loading, begin_world_load_at, chrono_date, load_player_vehicle,
    resolve_session_config, spawn_debug_overlay, LoadAssets, SessionRequest, WorldLoadJob,
    WorldStats,
};
use crate::in_game::stream_budget::FrameBudget;
use crate::in_game::tile_loading;
use crate::in_game::types::{
    CancelLoadingBtn, InGameEntity, InGameSessionInfo, LoadStage, LoadingScreenRoot,
    SimulationLoadState,
};
use crate::in_game::vehicle_spawner::{spawn_camera_rig, spawn_free_camera};
use crate::in_game::world::spawn_atmosphere_and_lighting;
use crate::new_game_screen::types::{GameCatalogue, NewGameSetup};
use crate::session_vehicles::{EntrySel, SessionVehicles, SpawnParams, SpawnPoint};
use crate::AppArgs;
use opts_bevy::{CameraMode, CameraState};

/// Everything the stage functions read and write.
#[derive(SystemParam)]
pub struct LoadEnv<'w, 's> {
    commands: Commands<'w, 's>,
    setup: Res<'w, NewGameSetup>,
    args: Option<Res<'w, AppArgs>>,
    catalogue: Option<Res<'w, GameCatalogue>>,
    service: Option<Res<'w, CatalogueService>>,
    assets: LoadAssets<'w>,
    job: Option<ResMut<'w, WorldLoadJob>>,
    stats: ResMut<'w, WorldStats>,
    camera_state: ResMut<'w, CameraState>,
    session: ResMut<'w, InGameSessionInfo>,
    report: ResMut<'w, LoadReport>,
    accept: ResMut<'w, AcceptLoadProblems>,
    accept_always: Option<Res<'w, crate::in_game::load_report::AcceptLoadProblemsAlways>>,
    registry: ResMut<'w, SessionVehicles>,
    settings: Option<Res<'w, crate::settings::GameSettings>>,
    hardware: Option<Res<'w, crate::hardware::HardwareState>>,
}

/// Progress of the stages, as fractions of the bar.
const P_TERRAIN: f32 = 0.1;
const P_TERRAIN_SPAN: f32 = 0.55;
const P_LIGHT: f32 = 0.7;
const P_VEHICLE: f32 = 0.9;

/// Advances the loading state machine. Progress reflects tiles actually parsed and spawned, not
/// elapsed time.
pub fn update_loading_progress(
    time: Res<Time>,
    mut state: ResMut<SimulationLoadState>,
    mut env: LoadEnv,
    theme: Res<MaterialTheme>,
    loading_screen: Query<Entity, With<LoadingScreenRoot>>,
    session_entities: Query<Entity, (With<InGameEntity>, Without<LoadingScreenRoot>)>,
) {
    if state.hud_spawned {
        return;
    }
    if env.report.retry_requested {
        restart(&mut env, &mut state, &session_entities);
        return;
    }
    // Headless runs (CI, tests) have no meaningful frame time; use a fixed step so the
    // ready-delay elapses. Loading itself is real work and does not depend on this value.
    let headless = env.args.as_ref().map(|a| a.0.headless).unwrap_or(true);
    let dt = if headless {
        0.35
    } else {
        time.delta_secs().min(0.1)
    };
    state.stage_elapsed += dt;
    match state.stage {
        LoadStage::InitEngine => stage_init(&mut env, &mut state),
        LoadStage::BuildTerrain => stage_terrain(&mut env, &mut state),
        LoadStage::SetupAtmosphere => stage_atmosphere(&mut env, &mut state),
        LoadStage::SpawnVehicle => stage_vehicle(&mut env, &mut state),
        LoadStage::CalibrateControls => stage_controls(&mut env, &mut state),
        LoadStage::Ready => stage_ready(&mut env, &mut state, dt, &theme, &loading_screen),
        LoadStage::Failed => {}
    }
}

/// Settles the report as failed and parks the state machine.
fn fail(env: &mut LoadEnv, state: &mut SimulationLoadState) {
    env.report.finish();
    state.stage = LoadStage::Failed;
}

/// First run: the menus and the resolution need the catalogue, which is still being built.
fn catalogue_pending(env: &LoadEnv) -> bool {
    let empty = env.catalogue.as_ref().is_none_or(|c| c.maps.is_empty());
    empty
        && env
            .service
            .as_ref()
            .is_some_and(|s| matches!(s.status(), DiscoveryStatus::Running(_)))
}

mod init;
use init::stage_init;

fn stage_terrain(env: &mut LoadEnv, state: &mut SimulationLoadState) {
    let LoadEnv {
        commands,
        assets,
        job,
        stats,
        ..
    } = env;
    let Some(job) = job.as_deref_mut() else {
        state.stage = LoadStage::SetupAtmosphere;
        return;
    };
    if job.has_tile_work() {
        // A budget per frame, not a whole tile: the window keeps answering while a dense tile
        // builds (M-0089).
        let mut budget = FrameBudget::new(tile_loading::LOADING_BUDGET);
        advance_tile_loading(
            job,
            stats,
            commands,
            &mut assets.meshes,
            &mut assets.materials,
            &mut assets.images,
            assets.ground_materials.as_deref_mut(),
            &mut budget,
            usize::MAX,
        );
    }
    (state.tiles_done, state.tiles_total) = job.tile_counts();
    state.progress = P_TERRAIN + P_TERRAIN_SPAN * job.progress();
    if job.tiles_done() {
        state.world_spawned = true;
        if env.stats.tiles_loaded == 0 {
            let name = env.stats.map_name.clone();
            env.report.add(
                LoadProblem::new(
                    true,
                    "load-problem-map",
                    &name,
                    "no map tile could be loaded",
                )
                .because("load-cause-no-tiles"),
            );
            fail(env, state);
            return;
        }
        state.stage = LoadStage::SetupAtmosphere;
    }
}

fn stage_atmosphere(env: &mut LoadEnv, state: &mut SimulationLoadState) {
    if !state.atmosphere_spawned {
        let LoadEnv {
            commands,
            assets,
            job,
            ..
        } = env;
        let sky = job.as_deref().map(|j| {
            super::sky::spawn_sky(
                commands,
                &mut assets.meshes,
                &mut assets.materials,
                &mut assets.images,
                j.omsi_root(),
            )
        });
        match sky {
            Some(Ok(())) => {}
            other => {
                if let Some(Err(e)) = other {
                    tracing::error!(error = %e, "sky unavailable; using default sun light");
                }
                spawn_atmosphere_and_lighting(commands);
            }
        }
        state.atmosphere_spawned = true;
    }
    state.progress = P_LIGHT;
    state.stage = LoadStage::SpawnVehicle;
}

fn stage_vehicle(env: &mut LoadEnv, state: &mut SimulationLoadState) {
    if !state.vehicle_spawned {
        let LoadEnv {
            commands,
            assets,
            job,
            stats,
            report,
            setup,
            registry,
            ..
        } = env;
        if let Some(job) = job.as_deref_mut().filter(|j| !j.no_vehicle) {
            // A train composed in the wizard: the cars behind the chosen vehicle.
            let formation = crate::new_game_screen::apply_formation_at_start(setup, job);
            let params = SpawnParams {
                catalogue_id: job
                    .vehicle_file()
                    .and_then(|p| p.file_stem())
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                name: setup.selected_vehicle.clone(),
                bus_file: job
                    .vehicle_file()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
                repaint: job.config_livery().to_owned(),
                setvars: job.config_setvars().to_vec(),
                entry: job
                    .start_entry()
                    .map_or(EntrySel::Default, |(_, n)| EntrySel::Name(n.to_owned())),
                formation,
            };
            let id = registry.request_spawn(params);
            match load_player_vehicle(
                job,
                stats,
                commands,
                &mut assets.meshes,
                &mut assets.materials,
                &mut assets.images,
                &mut assets.bindposes,
                id,
                true,
            ) {
                Ok(spawned) => {
                    let point = job.spawn_point().map(|(p, heading)| {
                        let abs = job.local_to_absolute(p.truncate());
                        SpawnPoint {
                            x: abs.x,
                            y: abs.y,
                            z: f64::from(p.z),
                            heading_deg: f64::from(heading),
                        }
                    });
                    let entry = job.start_entry().map(|(i, n)| (i, n.to_owned()));
                    if let Some(r) = registry.get_mut(id) {
                        r.name.clone_from(&spawned.name);
                    }
                    registry.mark_alive(id, spawned.root, entry, point, Some(spawned.camera));
                    registry.set_active(Some(id));
                }
                Err(e) => {
                    tracing::error!("cannot load player vehicle: {e}");
                    registry.mark_failed(id, e.clone());
                    report.add(
                        LoadProblem::new(false, "load-problem-vehicle", &setup.selected_vehicle, e)
                            .because("load-cause-vehicle-unreadable"),
                    );
                }
            }
        }
        state.vehicle_spawned = true;
    }
    state.progress = P_VEHICLE;
    state.stage = LoadStage::CalibrateControls;
}

/// The camera mode a `--camera` argument asks for, if any.
fn requested_camera(args: Option<&AppArgs>) -> Option<String> {
    args.and_then(|a| a.0.camera.as_deref())
        .map(str::to_ascii_lowercase)
}

fn stage_controls(env: &mut LoadEnv, state: &mut SimulationLoadState) {
    if !state.camera_spawned {
        let named = requested_camera(env.args.as_deref());
        if let Some(job) = env.job.as_deref() {
            setup_camera(
                &mut env.commands,
                job,
                &mut env.camera_state,
                named.as_deref(),
            );
        }
        spawn_debug_overlay(&mut env.commands);
        state.camera_spawned = true;
    }
    let (map, c) = (env.stats.map_name.clone(), counts(env, state));
    let failures = env
        .job
        .as_deref()
        .map(|j| j.failed_files(8).join("\n"))
        .unwrap_or_default();
    problems_from_counts(&mut env.report, &map, c, &failures);
    env.report.finish();
    env.stats.load_problems = env.report.count();
    state.progress = 1.0;
    state.stage = LoadStage::Ready;
    state.is_complete = true;
}

fn counts(env: &LoadEnv, state: &SimulationLoadState) -> WorldCounts {
    WorldCounts {
        tiles_loaded: env.stats.tiles_loaded,
        tiles_missing: env.stats.tiles_missing,
        tiles_total: state.tiles_total
            + env
                .job
                .as_deref()
                .map_or(0, WorldLoadJob::tiles_listed_missing),
        objects_failed: env.stats.objects_failed + env.stats.splines_failed,
        asset_failures: env.stats.asset_failures,
    }
}

/// Chooses the starting camera and mode from the loaded vehicle (or an inspection camera).
fn setup_camera(
    commands: &mut Commands,
    job: &WorldLoadJob,
    camera_state: &mut CameraState,
    named: Option<&str>,
) {
    if let Some(cam) = job.camera {
        spawn_camera_rig(commands, &cam);
        camera_state.mode = match named {
            Some("cockpit") => {
                camera_state.sub_camera_index = job.camera_std_index;
                CameraMode::Cockpit
            }
            Some("pax") => CameraMode::Passenger,
            Some("ego") => CameraMode::Ego,
            Some("free") => free_roam_start(commands, camera_state, &cam),
            _ => CameraMode::External,
        };
    } else if let Some(tf) = job.inspection_camera() {
        // No vehicle: still give the user a camera to inspect the loaded world.
        tracing::warn!("no vehicle loaded; starting in free-roam over the spawn point");
        spawn_free_camera(commands, tf);
        camera_state.mode = CameraMode::FreeRoam;
        camera_state.free_roam_spawn = tf.translation;
        let (yaw, pitch, _) = tf.rotation.to_euler(EulerRot::YXZ);
        camera_state.free_roam_yaw = yaw;
        camera_state.free_roam_pitch = pitch;
    }
    camera_state.previous_mode = camera_state.mode;
    tracing::info!(mode = ?camera_state.mode, "initial camera mode");
}

/// Free-roam start. Developer aid: `OPTS_DEV_CAM_HEIGHT=<m>` starts high above the spawn,
/// looking down (overview screenshots).
fn free_roam_start(
    commands: &mut Commands,
    camera_state: &mut CameraState,
    cam: &crate::in_game::omsi_world::CameraSetup,
) -> CameraMode {
    let mut start = cam.start;
    if let Some(h) = std::env::var("OPTS_DEV_CAM_HEIGHT")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
    {
        let focus = cam.start.translation + *cam.start.forward() * h * 0.5;
        start = Transform::from_translation(cam.start.translation + Vec3::Y * h).looking_at(
            Vec3::new(focus.x, cam.start.translation.y, focus.z),
            Vec3::Y,
        );
        commands.queue(move |w: &mut World| {
            let mut q = w.query_filtered::<&mut Transform, With<opts_bevy::MainCamera>>();
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
}

fn stage_ready(
    env: &mut LoadEnv,
    state: &mut SimulationLoadState,
    dt: f32,
    theme: &MaterialTheme,
    loading_screen: &Query<Entity, With<LoadingScreenRoot>>,
) {
    let go = match env.report.outcome {
        LoadOutcome::Ready => {
            state.auto_launch_delay -= dt;
            state.auto_launch_delay <= 0.0
        }
        LoadOutcome::ReadyWithProblems => {
            let by_flag = env.args.as_ref().is_some_and(|a| a.0.accept_load_problems);
            let always = env.accept_always.as_ref().is_some_and(|a| a.0);
            let accepted = by_flag || env.accept.0 || always;
            if accepted && !env.report.continue_requested {
                tracing::warn!(
                    problems = env.report.blocking_count(),
                    "starting the drive although the load had problems (accepted):\n{}",
                    env.report.as_text()
                );
            }
            if accepted {
                // The protocol's one-load acceptance is used up; the load counts as continued.
                env.accept.0 = false;
                env.report.continue_requested = true;
            }
            env.report.continue_requested || accepted
        }
        LoadOutcome::Loading | LoadOutcome::Failed => false,
    };
    if go {
        transition_to_active_simulation(
            &mut env.commands,
            state,
            &env.session,
            theme,
            loading_screen,
        );
    }
}

/// Which outcome button an entity is.
type OutcomeButton = (
    Has<DetailsBtn>,
    Has<ContinueBtn>,
    Has<RetryBtn>,
    Has<ChooseAnotherBtn>,
    Has<LoadingBackBtn>,
);

/// Handles the buttons of the loading screen: cancel, details, continue, retry, choose another,
/// back.
#[allow(clippy::too_many_arguments)]
pub fn handle_loading_interactions(
    mut activated: MessageReader<Activated>,
    cancel: Query<(), With<CancelLoadingBtn>>,
    buttons: Query<OutcomeButton>,
    mut report: ResMut<LoadReport>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for Activated(e) in activated.read() {
        if cancel.contains(*e) {
            // The dialog asks first; see `loading_ui::ask_to_cancel`.
            continue;
        }
        let Ok((details, cont, retry, another, back)) = buttons.get(*e) else {
            continue;
        };
        if details {
            report.details_open = !report.details_open;
        } else if cont {
            report.continue_requested = true;
        } else if retry {
            report.retry_requested = true;
        } else if another {
            next_state.set(AppState::VehicleSelection);
        } else if back {
            next_state.set(AppState::MainMenu);
        }
    }
}

/// Starts the load again on the same screen: the half-built world goes, the report is cleared.
fn restart(
    env: &mut LoadEnv,
    state: &mut SimulationLoadState,
    session_entities: &Query<Entity, (With<InGameEntity>, Without<LoadingScreenRoot>)>,
) {
    for e in session_entities {
        env.commands.entity(e).despawn();
    }
    env.commands.remove_resource::<WorldLoadJob>();
    env.commands.remove_resource::<super::sky::Sky>();
    env.registry.clear();
    *state = SimulationLoadState::default();
    env.report.reset();
}

/// Transitions from the loading screen to active gameplay: despawns loading overlay and spawns the driving HUD.
fn transition_to_active_simulation(
    commands: &mut Commands,
    state: &mut SimulationLoadState,
    session: &InGameSessionInfo,
    theme: &MaterialTheme,
    loading_screen_query: &Query<Entity, With<LoadingScreenRoot>>,
) {
    for entity in loading_screen_query {
        commands.entity(entity).despawn();
    }
    if !state.hud_spawned {
        spawn_in_game_hud(commands, theme, session);
        state.hud_spawned = true;
        let mips = opts_bevy::texture::mip_stats();
        tracing::info!(
            textures = mips.textures,
            millis = mips.micros / 1000,
            dropped_dds_chains = opts_bevy::texture::dropped_dds_chains(),
            "generated mip chains"
        );
        let bc = opts_bevy::texture::bc_chain_stats();
        tracing::info!(
            built = bc.built,
            from_cache = bc.cached,
            millis = bc.micros / 1000,
            "block-compressed mip chains"
        );
        tracing::info!("Simulation viewport active; driving HUD instantiated");
    }
}
