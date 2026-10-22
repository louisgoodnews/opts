//! The scripts of the units behind a head and the control cables between them (M-0098).

use std::time::Duration;

use bevy::{prelude::*, time::TimeUpdateStrategy};
use opts_bevy::{
    vehicle_physics_step_system, ConsistUnit, PassengerFeed, PlayerVehicle, ScriptConsist,
    ScriptPart, ScriptStartup, VehicleController, VehicleScriptPlugin, VehicleScripts,
};
use opts_sim::script::consist::Cable;
use opts_sim::script::ScriptEngine;

fn engine(script: &str) -> ScriptEngine {
    let (mut e, diags) = ScriptEngine::from_sources(&[("t.osc", script)]);
    assert!(diags.is_empty(), "{diags:?}");
    e.set("line", 1.0);
    e
}

fn cable(front: bool) -> Cable {
    Cable {
        front,
        kind: 'C',
        index: 0,
        receive: "bat_r".into(),
        send: "bat_w".into(),
        coupling: "line".into(),
    }
}

/// A head that switches the battery on and two cars behind it that only read it, one of them
/// with its own passenger door script.
fn consist() -> (App, Entity, [Entity; 2]) {
    let mut app = App::new();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
        1.0 / 60.0,
    )));
    app.add_plugins(MinimalPlugins)
        .add_plugins(VehicleScriptPlugin)
        .add_systems(FixedUpdate, vehicle_physics_step_system);
    let car = |app: &mut App, script: &str, feed: Option<PassengerFeed>| {
        let mut e = app
            .world_mut()
            .spawn((VehicleScripts::new(engine(script)), ScriptPart));
        if let Some(f) = feed {
            e.insert(f);
        }
        e.id()
    };
    let b = car(
        &mut app,
        "{frame}\n(L.L.bat_r) (S.L.b_sees)\n(L.L.PAX_Entry0_Req) (S.L.b_req)\n{end}",
        Some(PassengerFeed {
            entry_req: [true, false, false, false, false, false, false, false],
            ..Default::default()
        }),
    );
    let c = car(&mut app, "{frame}\n(L.L.bat_r) (S.L.c_sees)\n{end}", None);
    let units = vec![
        ConsistUnit {
            vehicle: 0,
            part: None,
            head: true,
            cables: vec![cable(false)],
        },
        ConsistUnit {
            vehicle: 0,
            part: Some(b),
            head: false,
            cables: vec![cable(true), cable(false)],
        },
        ConsistUnit {
            vehicle: 0,
            part: Some(c),
            head: false,
            cables: vec![cable(true)],
        },
    ];
    let mut controller = VehicleController::standard_bus(11_500.0, 5.85);
    controller.inputs_mut().set_parking_brake(true);
    let head = app
        .world_mut()
        .spawn((
            controller,
            VehicleScripts::new(engine("{frame}\n1 (S.L.bat_w)\n{end}")),
            ScriptStartup::default(),
            ScriptConsist { units },
            PlayerVehicle,
        ))
        .id();
    (app, head, [b, c])
}

fn get(app: &App, e: Entity, name: &str) -> f32 {
    let scripts = app.world().get::<VehicleScripts>(e).expect("scripts");
    let v = scripts.engine.lock().expect("engine").get(name);
    v
}

#[test]
fn what_the_head_puts_on_a_cable_reaches_the_cars_behind_it() {
    let (mut app, head, [b, c]) = consist();
    for _ in 0..6 {
        app.update();
    }
    assert_eq!(get(&app, head, "bat_r"), 1.0, "the head hears itself");
    assert_eq!(get(&app, b, "b_sees"), 1.0);
    assert_eq!(get(&app, c, "c_sees"), 1.0, "through the car between");
}

#[test]
fn a_part_hears_its_own_door_requests_and_not_the_heads() {
    let (mut app, _, [b, _]) = consist();
    for _ in 0..4 {
        app.update();
    }
    assert_eq!(get(&app, b, "b_req"), 1.0);
}
