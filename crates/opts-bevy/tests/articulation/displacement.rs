//! Leader displacement and jumps: the segment does not jackknife or spin.

use super::*;

/// Only alpha is exercised: beta and gamma need terrain in the harness (deferred to Phase E).
#[test]
pub(super) fn spawn_displacement_of_the_leader_does_not_jackknife_the_segment() {
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
pub(super) fn a_leader_jump_does_not_spin_the_segment_wheels() {
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
