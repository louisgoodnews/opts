//! A vehicle's state round trips with its script variables.


use super::*;


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
