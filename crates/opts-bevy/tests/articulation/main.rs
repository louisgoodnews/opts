//! A following segment trails its leader through a curve and the joint angle reaches the scripts.

use std::time::Duration;

use bevy::{prelude::*, time::TimeUpdateStrategy};
use opts_bevy::{
    vehicle_physics_step_system, ArticulatedChain, ArticulatedSegment, ArticulationPlugin,
    VehicleController, VehicleScripts,
};
use opts_sim::script::ScriptEngine;
use opts_sim::vehicle::inputs::GearPosition;

fn app() -> App {
    let mut app = App::new();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
        1.0 / 60.0,
    )));
    app.add_plugins(MinimalPlugins)
        .add_plugins(TransformPlugin)
        .add_plugins(ArticulationPlugin)
        .add_systems(FixedUpdate, vehicle_physics_step_system);
    app
}

fn spawn_articulated(app: &mut App, steering: f32) -> (Entity, Entity) {
    spawn_articulated_with(app, steering, false)
}

fn spawn_articulated_with(app: &mut App, steering: f32, reversed: bool) -> (Entity, Entity) {
    let mut controller = VehicleController::standard_bus(11_500.0, 5.85);
    controller.inputs_mut().set_parking_brake(false);
    controller.inputs_mut().set_gear(GearPosition::Drive);
    controller.inputs_mut().set_throttle(0.5);
    controller.inputs_mut().set_steering(steering);
    let (engine, _) = ScriptEngine::from_sources(&[("t.osc", "{frame}\n{end}")]);
    let mut seg = ArticulatedSegment::new(Vec2::new(0.0, -5.0), Vec2::new(0.0, 1.0), -5.0, 52.5);
    seg.reversed = reversed;
    // One trailer axle numbered after the head's two, wheel radius 0.5 m.
    seg.set_wheels(2, [0.5]);
    let segment = app
        .world_mut()
        .spawn((
            // Hitch 1.5 m behind the leader's origin; front coupling 1 m ahead of the segment's
            // origin; rear axle 6 m behind that.
            seg,
            Transform::default(),
        ))
        .id();
    let root = app
        .world_mut()
        .spawn((
            controller,
            Transform::default(),
            ArticulatedChain {
                segments: vec![segment],
            },
            VehicleScripts::new(engine),
        ))
        .id();
    (root, segment)
}

mod coupling;

mod displacement;

mod limits;

mod published;

/// Moves the leader by `by` metres in one step (a spawn placement, a collision push-out, a reset).
fn teleport(app: &mut App, root: Entity, by: Vec2) {
    let mut e = app.world_mut().entity_mut(root);
    let mut c = e.get_mut::<VehicleController>().unwrap();
    c.sim.chassis.position += by;
}
