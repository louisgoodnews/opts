//! Which drive a vehicle gets (M-0097): its scripts' torque, the const file's torque curve when
//! the scripts never hand any to the wheels, or the built-in constants when there is nothing to
//! hand torque at all.

use std::time::Duration;

use bevy::{prelude::*, time::TimeUpdateStrategy};
use opts_bevy::{
    vehicle_physics_step_system, PlayerVehicle, ScriptStartup, VehicleController,
    VehicleScriptPlugin, VehicleScripts,
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

fn vehicle(app: &mut App, script: &str) -> Entity {
    let mut controller = VehicleController::standard_bus(11_500.0, 5.85);
    controller.inputs_mut().set_parking_brake(false);
    controller.inputs_mut().set_throttle(1.0);
    let (mut engine, _diags) = ScriptEngine::from_sources(&[("t.osc", script)]);
    engine.run_init();
    app.world_mut()
        .spawn((
            controller,
            VehicleScripts::new(engine),
            ScriptStartup::default(),
            PlayerVehicle,
        ))
        .id()
}

fn script_drive(app: &App, e: Entity) -> Option<opts_sim::vehicle::ScriptDrive> {
    app.world()
        .get::<VehicleController>(e)
        .expect("controller")
        .sim
        .script_drive
}

fn run(app: &mut App, seconds: f32) {
    for _ in 0..(seconds * 60.0) as usize {
        app.update();
    }
}

#[test]
fn a_script_that_writes_torque_drives_the_vehicle() {
    let mut app = app();
    let e = vehicle(
        &mut app,
        "{frame}\n1 (S.L.engine_on)\n3000 (S.L.m_wheel)\n{end}",
    );
    run(&mut app, 8.0);
    let d = script_drive(&app, e).expect("script drive");
    assert!((d.wheel_torque_nm - 3000.0 * opts_sim::vehicle::M_WHEEL_UNIT).abs() < 1.0);
}

#[test]
fn a_script_that_never_hands_torque_under_throttle_falls_back_to_the_const_file() {
    let mut app = app();
    let e = vehicle(
        &mut app,
        "{frame}\n1 (S.L.engine_on)\n0 (S.L.m_wheel)\n{end}",
    );
    run(&mut app, 2.0);
    assert!(
        script_drive(&app, e).is_some(),
        "not yet: the script gets its time"
    );
    run(&mut app, 6.0);
    assert!(
        script_drive(&app, e).is_none(),
        "built-in torque curve drives it"
    );
}

#[test]
fn a_vehicle_whose_scripts_write_no_torque_uses_the_built_in_constants() {
    let mut app = app();
    let quiet = vehicle(&mut app, "{frame}\n1 (S.L.something)\n{end}");
    run(&mut app, 2.0);
    assert!(script_drive(&app, quiet).is_none());
}

#[test]
fn a_script_that_does_not_compile_leaves_the_built_in_drive() {
    let mut app = app();
    let broken = vehicle(&mut app, "{frame}\n((( (S.L.m_wheel\n{end}");
    run(&mut app, 2.0);
    assert!(script_drive(&app, broken).is_none());
}
