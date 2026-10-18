//! The contract between the game and a vehicle's scripts (M-0096): who owns a variable, in which
//! order the systems run, which vehicles run at all, and what `GetTime` gives after a long run.

use std::time::Duration;

use bevy::{prelude::*, time::TimeUpdateStrategy};
use opts_bevy::{
    vehicle_physics_step_system, PlayerVehicle, ScriptDriveEnabled, ScriptRadius, ScriptStartup,
    VehicleController, VehicleScriptPlugin, VehicleScripts,
};
use opts_sim::script::ScriptEngine;

fn app() -> App {
    let mut app = App::new();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
        1.0 / 60.0,
    )));
    app.add_plugins(MinimalPlugins)
        .add_plugins(VehicleScriptPlugin)
        .add_systems(FixedUpdate, vehicle_physics_step_system);
    app
}

fn vehicle(app: &mut App, script: &str, at: Vec2, player: bool) -> Entity {
    let mut controller = VehicleController::standard_bus(11_500.0, 5.85);
    controller.sim.chassis.position = at;
    controller.inputs_mut().set_parking_brake(true);
    let (mut engine, diags) = ScriptEngine::from_sources(&[("t.osc", script)]);
    assert!(diags.is_empty(), "{diags:?}");
    engine.run_init();
    let mut e = app.world_mut().spawn((
        controller,
        VehicleScripts::new(engine),
        ScriptStartup::default(),
    ));
    if player {
        e.insert(PlayerVehicle);
    }
    e.id()
}

fn get(app: &App, e: Entity, name: &str) -> f32 {
    let scripts = app.world().get::<VehicleScripts>(e).expect("scripts");
    let v = scripts.engine.lock().expect("engine").get(name);
    v
}

fn steps(app: &mut App, n: usize) {
    for _ in 0..n {
        app.update();
    }
}

#[test]
fn a_pedal_the_script_writes_keeps_its_value_and_one_it_does_not_is_the_drivers() {
    let mut app = app();
    // The script writes the throttle itself (an AI-style script); the driver's is 0.
    let writer = vehicle(
        &mut app,
        "{frame}\n0.4 (S.L.throttle)\n{end}",
        Vec2::ZERO,
        true,
    );
    // This one reads it and writes something else.
    let reader = vehicle(
        &mut app,
        "{frame}\n(L.L.throttle) 2 * (S.L.twice)\n{end}",
        Vec2::new(10.0, 0.0),
        false,
    );
    app.world_mut()
        .get_mut::<VehicleController>(reader)
        .expect("controller")
        .inputs_mut()
        .set_throttle(0.25);
    steps(&mut app, 4);
    assert!(
        (get(&app, writer, "throttle") - 0.4).abs() < 1e-6,
        "the script's pedal stays"
    );
    assert!(
        (get(&app, reader, "twice") - 0.5).abs() < 1e-6,
        "the driver's pedal is fed"
    );
}

#[test]
fn the_start_up_triggers_reach_the_same_frame_that_reads_them() {
    let mut app = app();
    app.insert_resource(ScriptDriveEnabled(true));
    let e = vehicle(
        &mut app,
        "{trigger:cp_batterietrennschalter_toggle}\n1 (S.L.flag)\n{end}\n\
         {frame}\n(L.L.flag) (S.L.seen)\n{end}",
        Vec2::ZERO,
        true,
    );
    for _ in 0..200 {
        app.update();
        if get(&app, e, "flag") > 0.5 {
            assert_eq!(
                get(&app, e, "seen"),
                1.0,
                "the frame after the trigger saw it in the same step"
            );
            return;
        }
    }
    panic!("the start-up never raised the trigger");
}

#[test]
fn only_the_players_vehicle_and_those_near_it_run() {
    let mut app = app();
    app.insert_resource(ScriptRadius(150.0));
    let script = "{frame}\n(L.L.n) 1 + (S.L.n)\n{end}";
    let player = vehicle(&mut app, script, Vec2::ZERO, true);
    let near = vehicle(&mut app, script, Vec2::new(100.0, 0.0), false);
    let far = vehicle(&mut app, script, Vec2::new(400.0, 0.0), false);
    steps(&mut app, 10);
    assert!(get(&app, player, "n") >= 5.0);
    assert!(get(&app, near, "n") >= 5.0, "within the radius");
    assert_eq!(
        get(&app, far, "n"),
        0.0,
        "outside it the scripts stand still"
    );
    // With nobody driving every vehicle runs.
    app.world_mut().entity_mut(player).remove::<PlayerVehicle>();
    steps(&mut app, 4);
    assert!(get(&app, far, "n") >= 2.0);
}

#[test]
fn get_time_keeps_its_resolution_after_a_long_run() {
    let (mut e, d) =
        ScriptEngine::from_sources(&[("t.osc", "{frame}\n(L.S.GetTime) (S.L.t)\n{end}")]);
    assert!(d.is_empty());
    // About 11.6 days of simulated time in ten steps: an f32 running total would have lost the
    // difference of two reads 16 ms apart.
    for _ in 0..10 {
        e.run_frame(100_000.0, false);
    }
    e.run_frame(0.016, false);
    let a = e.get("t");
    e.run_frame(0.016, false);
    let b = e.get("t");
    assert!(
        ((b - a) - 0.016).abs() < 0.004,
        "two reads 16 ms apart differ by {}",
        b - a
    );
    assert!(a < 65_536.0, "the value wraps");
}
