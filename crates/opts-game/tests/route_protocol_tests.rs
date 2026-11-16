//! `bus.routes`, `bus.get_route`, `bus.set_route` (M-0099): the depot's routes are listed, a
//! selection is keyed on the vehicle's IBIS one key per step and handed to the callbacks.

use std::sync::Arc;

use bevy::prelude::*;
use opts_bevy::control::channel::dispatch;
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_bevy::route::{route_typing_system, RouteHandle, VehicleRoute};
use opts_bevy::{PlayerVehicle, VehicleController, VehicleScripts};
use opts_cli::protocol::{code, RpcError};
use opts_core::formats::hof::HofFile;
use opts_core::formats::timetable::{parse_busstops, parse_line, parse_trip, Timetables};
use opts_sim::script::ScriptEngine;
use serde_json::{json, Value};

#[path = "route_protocol_tests/fixtures.rs"]
mod fixtures;
#[path = "route_protocol_tests/selection.rs"]
mod selection;
#[path = "route_protocol_tests/timetables.rs"]
mod timetables_tests;
use fixtures::{HOF, IBIS};

fn timetables() -> Timetables {
    Timetables::from_parts(
        parse_busstops(b"[busstop]\nPonttor\n1\n100\n0\n0\n\n[busstop]\nBushof\n1\n101\n0\n0\n\n"),
        vec![
            (
                "33".into(),
                parse_line(b"[userallowed]\n[priority]\n1\n[newtour]\nT1\nBus\n1\n[addtrip]\n33_V\n0\n480.000\n"),
            ),
            ("depot".into(), parse_line(b"[priority]\n1\n[newtour]\nD\nBus\n1\n")),
        ],
        vec![(
            "33_V".into(),
            parse_trip(b"[trip]\n\nVaals\n33\n[station_typ2]\n100\n[station_typ2]\n101\n[profile]\nstandard\n6.0\n[profile_man_arr_time]\n0\n0.0\n[profile_man_arr_time]\n1\n6.0\n"),
        )],
    )
}

fn app(script: &str) -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let mut registry = ControlRegistry::default();
    opts_game::control_route::register(&mut registry);
    app.insert_resource(registry);
    app.add_systems(Update, route_typing_system);
    let (mut engine, d) = ScriptEngine::from_sources(&[("t.osc", script)]);
    assert!(d.is_empty(), "{d:?}");
    engine.run_init();
    let hof = Arc::new(HofFile::parse_str(HOF).expect("hof"));
    let handle = RouteHandle::new(Some(hof));
    handle.set_timetables(Some(Arc::new(timetables())));
    let e = app
        .world_mut()
        .spawn((
            PlayerVehicle,
            VehicleController::standard_bus(11_500.0, 5.85),
            VehicleScripts::new(engine),
            VehicleRoute::new(handle),
        ))
        .id();
    (app, e)
}

fn call(app: &mut App, cmd: &str, p: Value) -> Result<Value, RpcError> {
    match dispatch(app.world_mut(), cmd, &p) {
        Outcome::Done(r) => r,
        Outcome::Wait(_) => panic!("answers at once"),
    }
}

fn var(app: &App, e: Entity, name: &str) -> f32 {
    let s = app.world().get::<VehicleScripts>(e).expect("scripts");
    let v = s.engine.lock().expect("engine").get(name);
    v
}
