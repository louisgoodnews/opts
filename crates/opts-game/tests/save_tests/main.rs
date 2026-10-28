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

#[test]
fn every_section_round_trips() {
    let mut app = session_world();
    {
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
    let file = capture(&mut app);
    for id in [
        "session",
        "time",
        "weather",
        "session_vehicles",
        "active_vehicle",
        "camera",
        "counters",
        "simulation",
    ] {
        assert!(file.sections.contains_key(id), "{id} not captured");
    }
    // The file survives a trip through text.
    let file = SaveFile::decode(&file.encode().expect("encode")).expect("decode");

    // A different session: everything the sections own differs.
    let mut other = session_world();
    {
        let w = other.world_mut();
        w.resource_mut::<CameraAvailability>().cockpit = true;
        w.resource_mut::<InGameSessionInfo>().sim_elapsed_seconds = 1.0;
    }
    let plan = restore(&mut other, &file);
    assert!(plan.warnings.is_empty() && plan.unknown.is_empty());
    let w = other.world();
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

#[test]
fn a_train_formation_survives_the_session_save() {
    use opts_core::formats::trains::{
        FormationSegment, FormationSource, Orientation, TrainFormation, VehicleRef,
    };
    let formation = TrainFormation {
        name: "Regio".into(),
        segments: vec![
            FormationSegment {
                vehicle: VehicleRef::File("vehicles\\BR218\\BR218_OR.ovh".into()),
                orientation: Orientation::Forward,
                powered: None,
            },
            FormationSegment {
                vehicle: VehicleRef::File("vehicles\\Bm238\\By438_mi.ovh".into()),
                orientation: Orientation::Backward,
                powered: Some(false),
            },
        ],
        source: FormationSource::ZugFile("Trains\\regio_grundorf.zug".into()),
    };
    let mut app = session_world();
    {
        let mut reg = app.world_mut().resource_mut::<SessionVehicles>();
        reg.request_spawn(SpawnParams {
            name: "Regio".into(),
            bus_file: "/nonexistent/BR218_OR.ovh".into(),
            formation: Some(formation.clone()),
            ..Default::default()
        });
        reg.request_spawn(SpawnParams {
            name: "Single".into(),
            bus_file: "/nonexistent/a.bus".into(),
            ..Default::default()
        });
    }
    let file = SaveFile::decode(&capture(&mut app).encode().expect("encode")).expect("decode");
    let mut other = session_world();
    restore(&mut other, &file);
    let records = other
        .world()
        .resource::<SessionVehicles>()
        .records()
        .to_vec();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].formation.as_ref(), Some(&formation));
    assert_eq!(records[1].formation, None, "a single vehicle stays single");
    assert_eq!(
        records[0].lifecycle,
        Lifecycle::Spawning,
        "the spawner builds the whole train"
    );
}

#[test]
fn a_vehicle_state_round_trips_with_its_script_variables() {
    let mut app = session_world();
    let (engine, _) = opts_sim::script::ScriptEngine::from_sources(&[("x.osc", "")]);
    let mut scripts = opts_bevy::vehicle_script::VehicleScripts::new(engine);
    scripts.initialised = true;
    scripts
        .engine
        .lock()
        .expect("engine")
        .set("door_front", 1.0);
    scripts
        .engine
        .lock()
        .expect("engine")
        .set_string("destination", "Spandau");
    let mut c = VehicleController::standard_bus(12_000.0, 5.9);
    c.sim.chassis.position = Vec2::new(120.5, -40.25);
    c.sim.chassis.heading_rad = 1.25;
    c.sim.chassis.elevation_z = 33.5;
    c.sim.chassis.local_velocity = Vec2::new(0.0, 8.0);
    c.inputs_mut().set_parking_brake(false);
    c.inputs_mut().set_throttle(0.4);
    c.inputs_mut()
        .set_gear(opts_sim::vehicle::inputs::GearPosition::Drive);
    let entity = app.world_mut().spawn((c, scripts)).id();
    let saved = state::capture(app.world(), entity).expect("state");
    assert_eq!(saved.script_floats, vec![("door_front".to_owned(), 1.0)]);

    let (engine2, _) = opts_sim::script::ScriptEngine::from_sources(&[("x.osc", "")]);
    let mut fresh = opts_bevy::vehicle_script::VehicleScripts::new(engine2);
    let other = app
        .world_mut()
        .spawn((VehicleController::standard_bus(12_000.0, 5.9), {
            fresh.initialised = false;
            fresh
        }))
        .id();
    assert!(
        !state::apply(app.world_mut(), other, &saved),
        "waits for the scripts' init"
    );
    app.world_mut()
        .get_mut::<opts_bevy::vehicle_script::VehicleScripts>(other)
        .expect("scripts")
        .initialised = true;
    assert!(state::apply(app.world_mut(), other, &saved));
    let again = state::capture(app.world(), other).expect("state");
    assert_eq!(again, saved);
    let ch = &app
        .world()
        .get::<VehicleController>(other)
        .expect("c")
        .sim
        .chassis;
    assert_eq!(
        (ch.position, ch.elevation_z, ch.local_velocity.y),
        (Vec2::new(120.5, -40.25), 33.5, 8.0)
    );
}

mod compatibility;
mod environment;
mod section_registry;
