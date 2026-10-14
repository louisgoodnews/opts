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

#[test]
fn trailer_wheels_roll_with_the_segments_own_travel() {
    for reversed in [false, true] {
        let mut app = app();
        let (root, _) = spawn_articulated_with(&mut app, 0.0, reversed);
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
        // Axle 2 is the trailer's; both sides spin alike and the speed matches rolling at 0.5 m.
        let spin = engine.get("wheel_rotation_2_l");
        assert_eq!(spin, engine.get("wheel_rotation_2_r"));
        let rpm = engine.get("wheel_rotationspeed_2_l");
        let speed = app
            .world()
            .get::<VehicleController>(root)
            .unwrap()
            .sim
            .chassis
            .forward_speed_m_s();
        let expected = speed / 0.5 * 60.0 / std::f32::consts::TAU;
        // A segment hanging backwards rolls the other way along its model.
        let expected = if reversed { -expected } else { expected };
        assert!(
            (rpm - expected).abs() < expected.abs() * 0.1 + 1.0,
            "reversed={reversed}: {rpm} rpm, expected {expected}"
        );
        assert!(spin != 0.0);
    }
}

/// Moves the leader by `by` metres in one step (a spawn placement, a collision push-out, a reset).
fn teleport(app: &mut App, root: Entity, by: Vec2) {
    let mut e = app.world_mut().entity_mut(root);
    let mut c = e.get_mut::<VehicleController>().unwrap();
    c.sim.chassis.position += by;
}

/// Only alpha is exercised: beta and gamma need terrain in the harness (deferred to Phase E).
#[test]
fn spawn_displacement_of_the_leader_does_not_jackknife_the_segment() {
    let mut app = app();
    let (root, _) = spawn_articulated(&mut app, 0.0);
    app.world_mut()
        .get_mut::<VehicleController>(root)
        .unwrap()
        .inputs_mut()
        .set_throttle(0.0);
    app.update();
    app.update();
    // The game places the vehicle after the chain exists.
    teleport(&mut app, root, Vec2::new(4.0, -3.0));
    for _ in 0..10 {
        app.update();
    }
    let engine = app
        .world()
        .get::<VehicleScripts>(root)
        .unwrap()
        .engine
        .lock()
        .unwrap();
    for var in [
        "articulation_0_alpha",
        "articulation_0_beta",
        "articulation_0_gamma",
    ] {
        assert!(engine.get(var).abs() < 0.01, "{var} = {}", engine.get(var));
    }
}

#[test]
fn a_leader_jump_does_not_spin_the_segment_wheels() {
    let mut app = app();
    let (root, _) = spawn_articulated(&mut app, 0.0);
    for _ in 0..240 {
        app.update();
    }
    let rpm_of = |app: &App| {
        let e = app
            .world()
            .get::<VehicleScripts>(root)
            .unwrap()
            .engine
            .lock()
            .unwrap();
        (
            e.get("wheel_rotationspeed_2_l"),
            e.get("articulation_0_alpha"),
        )
    };
    let speed = app
        .world()
        .get::<VehicleController>(root)
        .unwrap()
        .sim
        .chassis
        .forward_speed_m_s();
    let plausible = speed / 0.5 * 60.0 / std::f32::consts::TAU * 1.5 + 1.0;
    teleport(&mut app, root, Vec2::new(3.0, 2.0));
    for _ in 0..30 {
        app.update();
        let (rpm, alpha) = rpm_of(&app);
        assert!(rpm.abs() <= plausible, "rpm {rpm} above {plausible}");
        assert!(alpha.abs() < 5.0, "alpha {alpha}");
    }
}

#[test]
fn beta_is_limited_to_the_coupling_character() {
    let mut seg = ArticulatedSegment::new(Vec2::ZERO, Vec2::ZERO, -5.0, 52.5);
    assert_eq!(seg.clamp_beta_deg(35.0), 35.0, "unlimited without limits");
    seg.beta_limits_deg = Some((-20.0, 20.0));
    assert_eq!(seg.clamp_beta_deg(35.0), 20.0);
    assert_eq!(seg.clamp_beta_deg(-35.0), -20.0);
    assert_eq!(seg.clamp_beta_deg(5.0), 5.0);
}

/// The bellows' follower end (behind the joint) must land where the follower's body puts it: a
/// follower pitched nose up by `p` lowers its front-rear points behind the joint, and the bellows
/// dummy (`anim_rot articulation_0_beta`, no origin rotation, factor 1) must do the same.
#[test]
fn bellows_beta_moves_the_follower_end_like_the_follower_body() {
    use opts_bevy::{animation_matrix, joint_beta_deg, AnimStep};
    let p = 8.0_f32.to_radians();
    let step = AnimStep {
        frame: Mat4::IDENTITY,
        variable: "articulation_0_beta".into(),
        factor: 1.0,
        translate: false,
        maxspeed: None,
    };
    let beta = joint_beta_deg(0.0, p);
    let end_by_bellows =
        animation_matrix(&[step], |_| beta).transform_point3(Vec3::new(0.0, 0.0, 2.0));
    // Bevy axes: -z forward, nose-up pitch is a positive rotation about x.
    let end_by_body = Quat::from_rotation_x(p) * Vec3::new(0.0, 0.0, 2.0);
    assert!(
        end_by_bellows.distance(end_by_body) < 1e-4,
        "bellows {end_by_bellows:?}, body {end_by_body:?}"
    );
}

/// Gamma (roll, right side up positive) uses the frame `origin_rot (0,90,90)` of the census: the
/// bellows end must roll like the body (no sign change needed).
#[test]
fn bellows_gamma_rolls_the_follower_end_like_the_follower_body() {
    use opts_bevy::{animation_matrix, AnimStep};
    let r = 6.0_f32.to_radians();
    let frame = Mat4::from_rotation_x(0.0)
        * Mat4::from_rotation_y(90.0_f32.to_radians())
        * Mat4::from_rotation_z(90.0_f32.to_radians());
    let step = AnimStep {
        frame,
        variable: "articulation_0_gamma".into(),
        factor: 1.0,
        translate: false,
        maxspeed: None,
    };
    let gamma = (r - 0.0).to_degrees();
    let by_bellows = animation_matrix(&[step], |_| gamma).transform_point3(Vec3::X);
    let by_body = Quat::from_rotation_z(r) * Vec3::X;
    assert!(
        by_bellows.distance(by_body) < 1e-4,
        "{by_bellows:?} vs {by_body:?}"
    );
}

/// Design V1 (F-002): the joint angle the scripts see is the simulation's own, at every fixed
/// step of an S-curve.
#[test]
fn the_published_alpha_is_the_simulations_joint_angle_at_every_step() {
    let mut app = app();
    let (root, _) = spawn_articulated(&mut app, 0.6);
    let mut worst: f32 = 0.0;
    for step in 0..600 {
        // S-curve: the steering flips every 150 steps.
        let steering = if (step / 150) % 2 == 0 { 0.6 } else { -0.6 };
        app.world_mut()
            .get_mut::<VehicleController>(root)
            .unwrap()
            .inputs_mut()
            .set_steering(steering);
        app.update();
        // The first update may not run a fixed step yet: nothing is seeded before that.
        let Some(sim_alpha) = app
            .world()
            .get::<VehicleController>(root)
            .unwrap()
            .sim
            .consist_alpha_rad(0)
            .map(|a| a.to_degrees() as f32)
        else {
            continue;
        };
        let published = app
            .world()
            .get::<VehicleScripts>(root)
            .unwrap()
            .engine
            .lock()
            .unwrap()
            .get("articulation_0_alpha");
        worst = worst.max((sim_alpha - published).abs());
    }
    assert!(
        worst < 1e-4,
        "published alpha differs from the simulation by {worst} deg"
    );
    let c = app.world().get::<VehicleController>(root).unwrap();
    assert_eq!(c.sim.consist.len(), 1);
}

/// An explicit `teleport` re-seeds the consist: no jackknife, no wheel spike.
#[test]
fn an_explicit_teleport_reseeds_the_consist() {
    let mut app = app();
    let (root, _) = spawn_articulated(&mut app, 0.6);
    for _ in 0..300 {
        app.update();
    }
    app.world_mut()
        .get_mut::<VehicleController>(root)
        .unwrap()
        .sim
        .teleport(Vec2::new(900.0, 900.0), 0.7);
    app.update();
    let engine = app
        .world()
        .get::<VehicleScripts>(root)
        .unwrap()
        .engine
        .lock()
        .unwrap();
    // One step of steering 0.6 after hanging straight: the angle is tiny, not the old curve's.
    assert!(engine.get("articulation_0_alpha").abs() < 2.0);
    assert!(engine.get("wheel_rotationspeed_2_l").abs() < 200.0);
}
