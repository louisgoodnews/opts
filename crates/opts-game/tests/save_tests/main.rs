//! Session save: round trip of every section, compatibility (old fixture, unknown and newer
//! sections), damaged and missing content, and that a section registered from a test needs no
//! change to the save system (`docs/src/architecture/session-save.md`).

use bevy::prelude::*;
use opts_bevy::vehicle_remote::RemoteControls;
use opts_bevy::{CameraAvailability, CameraMode, CameraState, VehicleController};
use opts_game::in_game::types::InGameSessionInfo;
use opts_game::new_game_screen::types::NewGameSetup;
use opts_game::pause::{Pause, SessionStats};
use opts_game::save::container::{SaveFile, SaveMeta};
use opts_game::save::registry::{
    plan_restore, preflight, CheckEnv, IssueKind, SaveAppExt, SaveRegistry, SaveSection, Stage,
};
use opts_game::save::{sections, PlayClock, SaveStore};
use opts_game::session_vehicles::{
    state, Lifecycle, SessionVehicles, SpawnParams, SAVE_SECTION_VERSION,
};
use serde_json::{json, Value};

fn dir(name: &str) -> std::path::PathBuf {
    let d = std::env::var_os("HOME")
        .map_or_else(std::env::temp_dir, std::path::PathBuf::from)
        .join(".local/share/opts/benchmark/session-save/tests")
        .join(name);
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn session_world() -> App {
    let mut app = App::new();
    app.init_resource::<SaveRegistry>()
        .init_resource::<NewGameSetup>()
        .init_resource::<InGameSessionInfo>()
        .init_resource::<SessionVehicles>()
        .init_resource::<CameraState>()
        .init_resource::<CameraAvailability>()
        .init_resource::<SessionStats>()
        .init_resource::<RemoteControls>()
        .init_resource::<Pause>()
        .init_resource::<PlayClock>()
        .init_resource::<Time<Virtual>>();
    sections::register(&mut app);
    app
}

fn capture(app: &mut App) -> SaveFile {
    let sections = app.world().resource::<SaveRegistry>().all().to_vec();
    let (blobs, failures) = SaveRegistry::capture_all(&sections, app.world_mut());
    assert!(failures.is_empty(), "{failures:?}");
    let mut file = SaveFile::new(SaveMeta::default());
    file.sections = blobs;
    file
}

/// Prepare, then every stage in order, as a restore does.
fn restore(app: &mut App, file: &SaveFile) -> opts_game::save::registry::RestorePlan {
    let sections = app.world().resource::<SaveRegistry>().all().to_vec();
    let plan = plan_restore(&sections, file);
    for (s, data) in plan.prepare_steps().into_iter().map(|p| (&p.0, &p.1)) {
        (s.prepare.expect("prepare"))(app.world_mut(), data).expect("prepare");
    }
    for stage in [Stage::World, Stage::Settled] {
        for (s, data) in plan.steps.iter().filter(|(s, _)| s.stage == stage) {
            if let Some(apply) = s.apply {
                apply(app.world_mut(), data).unwrap_or_else(|e| panic!("{}: {e}", s.id));
            }
        }
    }
    plan
}

/// Fills every resource the sections own with distinctive values.
fn set_session_state(app: &mut App) {
    let w = app.world_mut();
    let mut setup = w.resource_mut::<NewGameSetup>();
    setup.selected_map = "Berlin-Spandau".into();
    setup.selected_entry_point = "Depot".into();
    setup.weather_preset = "Rain".into();
    (setup.year, setup.month, setup.day, setup.hour, setup.minute) = (1990, 3, 4, 17, 45);
    let mut info = w.resource_mut::<InGameSessionInfo>();
    info.map_name = "Berlin-Spandau".into();
    info.entry_point = "Depot".into();
    info.start_hour = 17;
    info.start_minute = 45;
    info.sim_elapsed_seconds = 1234.5;
    w.resource_mut::<PlayClock>().played_s = 99.5;
    let mut cam = w.resource_mut::<CameraState>();
    cam.mode = CameraMode::Cockpit;
    cam.previous_mode = CameraMode::External;
    cam.sub_camera_index = 2;
    cam.external_zoom = 14.0;
    cam.view_yaw = 0.3;
    w.resource_mut::<CameraAvailability>().cockpit = true;
    let mut stats = w.resource_mut::<SessionStats>();
    (stats.duration_s, stats.distance_m, stats.max_speed_m_s) = (80.0, 912.0, 13.5);
    w.resource_mut::<RemoteControls>().odometer_m = 912.25;
    let mut time = w.resource_mut::<Time<Virtual>>();
    time.set_relative_speed(4.0);
    time.pause();
    let mut reg = w.resource_mut::<SessionVehicles>();
    reg.request_spawn(SpawnParams {
        name: "Bus A".into(),
        bus_file: "/nonexistent/a.bus".into(),
        ..Default::default()
    });
}

/// Asserts that `w` holds the values [`set_session_state`] wrote, as restored from a save.
fn assert_restored_session(w: &World) {
    let setup = w.resource::<NewGameSetup>();
    assert_eq!(
        (
            setup.selected_map.as_str(),
            setup.selected_entry_point.as_str()
        ),
        ("Berlin-Spandau", "Depot")
    );
    assert!(setup.no_vehicle, "vehicles come back through the spawner");
    assert_eq!(setup.weather_preset, "Rain");
    assert_eq!((setup.year, setup.month, setup.day), (1990, 3, 4));
    assert_eq!((setup.hour, setup.minute), (17, 45));
    assert_eq!(
        w.resource::<InGameSessionInfo>().sim_elapsed_seconds,
        1234.5
    );
    assert_eq!(w.resource::<PlayClock>().played_s, 99.5);
    let cam = w.resource::<CameraState>();
    assert_eq!(cam.mode, CameraMode::Cockpit);
    assert_eq!(cam.previous_mode, CameraMode::External);
    assert_eq!(
        (cam.sub_camera_index, cam.external_zoom, cam.view_yaw),
        (2, 14.0, 0.3)
    );
    let stats = w.resource::<SessionStats>();
    assert_eq!(
        (stats.duration_s, stats.distance_m, stats.max_speed_m_s),
        (80.0, 912.0, 13.5)
    );
    assert_eq!(w.resource::<RemoteControls>().odometer_m, 912.25);
    let time = w.resource::<Time<Virtual>>();
    assert!(time.is_paused());
    assert_eq!(time.relative_speed(), 4.0);
    let reg = w.resource::<SessionVehicles>();
    assert_eq!(reg.records().len(), 1);
    assert_eq!(
        reg.records()[0].lifecycle,
        Lifecycle::Spawning,
        "rebuilt by the spawner"
    );
}

mod round_trip;
mod vehicle;
mod compatibility;
mod environment;
mod section_registry;
