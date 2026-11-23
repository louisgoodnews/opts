//! The live trailer-front joint of a one-trailer articulated head follows that trailer's yaw.
//!
//! The joint is driven by `trailer_front_system` from the running simulation (not by the
//! `rigid` test helper), so this checks the index of the unit it reads: the coupled trailer is
//! unit 0 (`articulation_0_alpha`), and the joint must turn by the same angle. The sweep covers
//! both steer signs, two speeds, and flat, pitched, rolled and pitched-and-rolled ground. In every
//! case the joint must pivot about the hitch `H` (`H` is a fixed point of the joint).

use std::time::Duration;

use bevy::{math::DVec3, prelude::*, time::TimeUpdateStrategy};
use opts_bevy::articulation::{joint_model_transform, TrailerFrontJoint};
use opts_bevy::{
    omsi_to_bevy_pos, vehicle_physics_step_system, ArticulatedChain, ArticulatedSegment,
    ArticulationPlugin, VehicleController, VehicleScripts,
};
use opts_sim::script::ScriptEngine;
use opts_sim::vehicle::ground::GroundSample;
use opts_sim::vehicle::inputs::GearPosition;
use opts_sim::vehicle::trailer_front::joint_transform;

const HITCH: DVec3 = DVec3::new(0.0, -4.4, 0.4);
const STEERS: [f32; 6] = [0.2, 0.4, 0.6, -0.2, -0.4, -0.6];
const THROTTLES: [f32; 2] = [0.3, 0.5];
/// Ground slopes `(d h/d x, d h/d y)`: flat, pitch (along the heading), roll (across), both.
const GROUNDS: [(f32, f32); 4] = [(0.0, 0.0), (0.0, 0.05), (0.05, 0.0), (0.05, 0.05)];

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
fn spawn(app: &mut App, steering: f32, throttle: f32) -> (Entity, Entity) {
    let mut controller = VehicleController::standard_bus(11_500.0, 5.85);
    controller.inputs_mut().set_parking_brake(false);
    controller.inputs_mut().set_gear(GearPosition::Drive);
    controller.inputs_mut().set_throttle(throttle);
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
            TrailerFrontJoint { root, hitch: HITCH },
            Transform::IDENTITY,
        ))
        .id();
    (root, joint)
}

/// Settles the body of every unit on a plane of slope `(sx, sy)`, as the game's ground following
/// does over many fixed steps (the filter moves about a sixth of the way per step).
fn settle_ground(controller: &mut VehicleController, (sx, sy): (f32, f32)) {
    let samples: Vec<Option<GroundSample>> = controller
        .sim
        .ground_probes()
        .into_iter()
        .map(|p| {
            let h = p.points().map(|q| sx * q.x + sy * q.y);
            Some(GroundSample::from_heights(p, h))
        })
        .collect();
    for _ in 0..240 {
        controller.sim.apply_ground(&samples, 1.0 / 60.0);
    }
}

/// In a steady turn the joint's yaw (Bevy `Y`, the OMSI `Z` axis) equals the published
/// `articulation_0_alpha` of the coupled trailer, which is the sim's `consist_alpha_rad(0)`, and
/// the joint fixes the hitch.
#[test]
fn live_trailer_front_joint_turns_with_the_coupled_trailer_across_the_sweep() {
    let hitch_bevy = omsi_to_bevy_pos(HITCH.as_vec3());
    let mut cases = 0;
    for steering in STEERS {
        for throttle in THROTTLES {
            for ground in GROUNDS {
                let mut app = app();
                let (root, joint) = spawn(&mut app, steering, throttle);
                for _ in 0..900 {
                    app.update();
                }
                let speed = {
                    let mut controller =
                        app.world_mut().get_mut::<VehicleController>(root).unwrap();
                    settle_ground(&mut controller, ground);
                    controller.forward_speed_km_h()
                };
                app.update();
                let controller = app.world().get::<VehicleController>(root).unwrap();
                let alpha = controller.sim.consist_alpha_rad(0).expect("the trailer");
                let (beta, gamma) = controller.sim.joint_angles_deg(0).expect("the trailer");
                let tf = *app.world().get::<Transform>(joint).unwrap();
                let case = format!("steer {steering:+.1} throttle {throttle:.1} ground {ground:?}");
                let pivot = tf.transform_point(hitch_bevy).distance(hitch_bevy);
                println!(
                    "{case:<46} {speed:6.2} km/h alpha {:+7.3} deg beta {beta:+6.2} gamma {gamma:+6.2} pivot {pivot:.1e} m",
                    alpha.to_degrees()
                );
                assert!(
                    alpha.abs() > 0.02,
                    "{case}: the trailer did not turn: {alpha} rad"
                );
                assert!(
                    pivot < 1e-4,
                    "{case}: the joint moves the hitch by {pivot} m"
                );
                if ground == (0.0, 0.0) {
                    let yaw = Quat::from_rotation_y(alpha as f32);
                    assert!(
                        tf.rotation.angle_between(yaw) < 1e-3,
                        "{case}: {:?}",
                        tf.rotation
                    );
                }
                let expected = joint_model_transform(joint_transform(
                    HITCH,
                    -alpha,
                    f64::from(beta).to_radians(),
                    f64::from(gamma).to_radians(),
                ));
                let (rot, trans) = (
                    tf.rotation.angle_between(expected.rotation),
                    tf.translation.distance(expected.translation),
                );
                assert!(
                    rot < 1e-3 && trans < 1e-3,
                    "{case}: rot {rot:.2e} rad, trans {trans:.2e} m"
                );
                cases += 1;
            }
        }
    }
    println!("trailer_front sweep: {cases} cases");
}
