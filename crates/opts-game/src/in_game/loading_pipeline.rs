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

mod stages;
use stages::{stage_controls, stage_vehicle};

mod outcome;
mod terrain;

pub use outcome::handle_loading_interactions;
use outcome::{restart, stage_ready};
use terrain::{stage_atmosphere, stage_terrain};
