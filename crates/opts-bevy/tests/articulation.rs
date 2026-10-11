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
    let mut controller = VehicleController::standard_bus(11_500.0, 5.85);
    controller.inputs_mut().set_parking_brake(false);
    controller.inputs_mut().set_gear(GearPosition::Drive);
    controller.inputs_mut().set_throttle(0.5);
    controller.inputs_mut().set_steering(steering);
    let (engine, _) = ScriptEngine::from_sources(&[("t.osc", "{frame}\n{end}")]);
    let segment = app
        .world_mut()
        .spawn((
            // Hitch 1.5 m behind the leader's origin; front coupling 1 m ahead of the segment's
            // origin; rear axle 6 m behind that.
            ArticulatedSegment::new(Vec2::new(0.0, -5.0), Vec2::new(0.0, 1.0), -5.0, 52.5),
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

#[test]
fn the_segment_stays_hitched_and_the_joint_angle_is_published() {
    let mut app = app();
    let (root, segment) = spawn_articulated(&mut app, 0.6);
    for _ in 0..600 {
        app.update();
    }
    let controller = app.world().get::<VehicleController>(root).unwrap();
    let chassis = &controller.sim.chassis;
    assert!(
        controller.forward_speed_km_h() > 3.0,
        "the vehicle did not move"
    );
    // The vehicle turns, so the segment lags against it: |α| clearly non-zero, within the limit.
    let engine = app
        .world()
        .get::<VehicleScripts>(root)
        .unwrap()
        .engine
        .lock()
        .unwrap();
    let alpha = engine.get("articulation_0_alpha");
    assert!(alpha.abs() > 3.0 && alpha.abs() <= 52.5, "α = {alpha}°");
    assert_eq!(engine.get("articulation_0_beta"), 0.0);
    drop(engine);

    // Front coupling of the segment sits on the rear coupling of the leader (flat, world space).
    let heading = f64::from(chassis.heading_rad);
    let leader =
        bevy::math::DVec2::new(f64::from(chassis.position.x), f64::from(chassis.position.y));
    let hitch = leader
        + bevy::math::DVec2::new(heading.cos(), -heading.sin()) * 0.0
        + bevy::math::DVec2::new(heading.sin(), heading.cos()) * -5.0;
    let tf = app.world().get::<Transform>(segment).unwrap();
    // Bevy → OMSI: x east, y = -z. The segment's heading comes from its rotation about Y.
    let (yaw, _, _) = tf.rotation.to_euler(EulerRot::YXZ);
    let seg_heading = -f64::from(yaw);
    let origin = bevy::math::DVec2::new(f64::from(tf.translation.x), -f64::from(tf.translation.z));
    let front = origin + bevy::math::DVec2::new(seg_heading.sin(), seg_heading.cos()) * 1.0;
    assert!(
        front.distance(hitch) < 0.05,
        "gap between the couplings: {} m",
        front.distance(hitch)
    );
}

#[test]
fn driving_straight_keeps_the_segment_in_line() {
    let mut app = app();
    let (root, _) = spawn_articulated(&mut app, 0.0);
    for _ in 0..300 {
        app.update();
    }
    let engine = app
        .world()
        .get::<VehicleScripts>(root)
        .unwrap()
        .engine
        .lock()
        .unwrap();
    assert!(engine.get("articulation_0_alpha").abs() < 0.5);
}
