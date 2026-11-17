//! Sign convention of `articulation_<k>_alpha` (Q-ART-01), pinned on the OPTS side.

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
    let seg = ArticulatedSegment::new(Vec2::new(0.0, -5.0), Vec2::new(0.0, 1.0), -5.0, 52.5);
    let segment = app.world_mut().spawn((seg, Transform::default())).id();
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

/// Sign convention of the published joint angle (Q-ART-01; the owner decided on 2026-11-17 that
/// OPTS publishes counter-clockwise positive), pinned with explicit expectations. World headings
/// are compass style (clockwise from north), a right-hand turn increases the heading, and the
/// segment lags in a curve. So in a right-hand curve `articulation_0_alpha` is POSITIVE and in a
/// left-hand curve NEGATIVE, and `consist_alpha_rad` returns the same sign. Whether OMSI uses the
/// same sign is not decided here (needs OMSI, see M-0053).
#[test]
fn published_alpha_is_positive_in_a_right_hand_curve_and_negative_in_a_left_hand_one() {
    for steering in [0.6_f32, -0.6] {
        let mut app = app();
        let (root, _) = spawn_articulated(&mut app, steering);
        for _ in 0..900 {
            app.update();
        }
        let controller = app.world().get::<VehicleController>(root).unwrap();
        let turned = controller.sim.chassis.heading_rad;
        let sim_alpha = controller.sim.consist_alpha_rad(0).expect("joint");
        let published = app
            .world()
            .get::<VehicleScripts>(root)
            .unwrap()
            .engine
            .lock()
            .unwrap()
            .get("articulation_0_alpha");
        assert!(turned.abs() > 0.3, "the vehicle did not turn: {turned} rad");
        let right_hand = turned > 0.0;
        assert!(
            (published > 3.0) == right_hand && (published < -3.0) == !right_hand,
            "steering {steering}: heading {turned} rad, alpha {published} deg"
        );
        assert_eq!(sim_alpha > 0.0, right_hand, "joint angle {sim_alpha} rad");
    }
}
