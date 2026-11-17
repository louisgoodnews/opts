//! The live trailer-front joint of a one-trailer articulated head follows that trailer's yaw.
//!
//! The joint is driven by `trailer_front_system` from the running simulation (not by the
//! `rigid` test helper), so this checks the index of the unit it reads: the coupled trailer is
//! unit 0 (`articulation_0_alpha`), and the joint must turn by the same angle, for both signs.

use std::time::Duration;

use bevy::{math::DVec3, prelude::*, time::TimeUpdateStrategy};
use opts_bevy::articulation::TrailerFrontJoint;
use opts_bevy::{
    vehicle_physics_step_system, ArticulatedChain, ArticulatedSegment, ArticulationPlugin,
    VehicleController, VehicleScripts,
};
use opts_sim::script::ScriptEngine;
use opts_sim::vehicle::ground::GroundSample;
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

/// A one-trailer articulated head with its trailer-front joint below the root.
fn spawn(app: &mut App, steering: f32) -> (Entity, Entity) {
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
    let joint = app
        .world_mut()
        .spawn((
            TrailerFrontJoint {
                root,
                hitch: DVec3::new(0.0, -4.4, 0.4),
            },
            Transform::IDENTITY,
        ))
        .id();
    (root, joint)
}

/// In a steady turn the joint's yaw (Bevy `Y`, the OMSI `Z` axis) equals the published
/// `articulation_0_alpha` of the coupled trailer, which is the sim's `consist_alpha_rad(0)`.
#[test]
fn live_trailer_front_joint_turns_with_the_coupled_trailer_for_both_signs() {
    for steering in [0.6_f32, -0.6] {
        let mut app = app();
        let (root, joint) = spawn(&mut app, steering);
        for _ in 0..900 {
            app.update();
        }
        // Flat ground under every unit: the bodies (pitch and roll zero) that the joint angles
        // need, as the game's ground following provides them.
        {
            let mut controller = app.world_mut().get_mut::<VehicleController>(root).unwrap();
            let samples: Vec<Option<GroundSample>> = controller
                .sim
                .ground_probes()
                .into_iter()
                .map(|p| Some(GroundSample::from_heights(p, [0.0; 4])))
                .collect();
            controller.sim.apply_ground(&samples, 1.0 / 60.0);
        }
        app.update();
        let controller = app.world().get::<VehicleController>(root).unwrap();
        let alpha = controller.sim.consist_alpha_rad(0).expect("the trailer");
        assert!(
            alpha.abs() > 0.3,
            "steering {steering}: the trailer did not turn: {alpha} rad"
        );
        let rotation = app.world().get::<Transform>(joint).unwrap().rotation;
        let expected = Quat::from_rotation_y(alpha as f32);
        assert!(
            rotation.angle_between(expected) < 1e-3,
            "steering {steering}: joint {rotation:?} does not follow the trailer's yaw \
             {alpha} rad (expected {expected:?})"
        );
    }
}
