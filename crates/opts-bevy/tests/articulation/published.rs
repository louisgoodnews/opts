//! The published joint angle and the explicit teleport.

use super::*;

/// Design V1 (F-002): the joint angle the scripts see is the simulation's own, at every fixed
/// step of an S-curve.
#[test]
pub(super) fn the_published_alpha_is_the_simulations_joint_angle_at_every_step() {
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
pub(super) fn an_explicit_teleport_reseeds_the_consist() {
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
