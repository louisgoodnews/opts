//! Unit tests of the vehicle simulation: ground following and ground state.

use super::*;

/// Acceptance 3 and the NG272 target in the harness: on flat ground pitch and roll stay 0;
/// on a constant grade every unit takes the grade angle and `β` is 0, and a unit that stands
/// on a different grade shows the difference of the two angles.
#[test]
fn fixed_step_ground_following_gives_zero_on_flat_and_the_grade_difference_on_a_step() {
    let mut sim = articulated();
    sim.inputs.set_throttle(0.0);
    sim.apply_ground(&samples_on_slope(&sim, 0.0), 1.0 / 64.0);
    let (beta, gamma) = sim.joint_angles_deg(0).unwrap();
    assert!(beta.abs() < 0.01 && gamma.abs() < 0.01);
    for _ in 0..200 {
        let s = samples_on_slope(&sim, 0.1);
        sim.apply_ground(&s, 1.0 / 64.0);
    }
    let grade_deg = 0.1_f32.atan().to_degrees();
    let (beta, _) = sim.joint_angles_deg(0).unwrap();
    // Chassis and unit both settle on the grade: β returns to 0 (the head was sampled on
    // flat ground first, so it has to catch up; after 3 s both agree).
    assert!(beta.abs() < 0.05, "{beta}");
    assert!((sim.consist_body(0).unwrap().pitch.to_degrees() - grade_deg).abs() < 0.05);
    // Unit on the grade, chassis still level: β is minus the grade angle (nose-up follower).
    let mut flat_head = articulated();
    let mut s = samples_on_slope(&flat_head, 0.0);
    s[1] = samples_on_slope(&flat_head, 0.1)[1];
    flat_head.apply_ground(&s, 1.0 / 64.0);
    let (beta, _) = flat_head.joint_angles_deg(0).unwrap();
    // The coupling sits on the level hitch, 7 m ahead of the rear axle, which stands on the grade
    // at height `0.1·y`: beta = asin(drop / 7) with the drop below the level hitch (rigid coupling model).
    let drop = f64::from(flat_head.ground_probes()[1].points()[0].y * 0.1);
    let expected = (drop / 7.0).asin().to_degrees() as f32;
    assert!(
        (beta - expected).abs() < 0.05,
        "{beta} vs {expected} (grade {grade_deg})"
    );
}

#[test]
fn ground_state_is_reseeded_by_teleport_and_serialised() {
    let mut sim = articulated();
    sim.apply_ground(&samples_on_slope(&sim, 0.05), 1.0 / 64.0);
    let back: VehicleSimulation = bincode::deserialize(&bincode::serialize(&sim).unwrap()).unwrap();
    assert_eq!(sim, back);
    assert!(sim.joint_angles_deg(0).is_some());
    sim.teleport(Vec2::new(10.0, 10.0), 0.0);
    assert!(
        sim.joint_angles_deg(0).is_none(),
        "no body until the next sample"
    );
    // A missing sample keeps the height and levels the tilt (see `apply_ground`).
    sim.apply_ground(&samples_on_slope(&sim, 0.05), 1.0 / 64.0);
    let kept = sim.consist_body(0).unwrap();
    sim.apply_ground(&[None, None], 1.0 / 64.0);
    let after = sim.consist_body(0).unwrap();
    assert_eq!(after.elevation, kept.elevation);
    assert!(after.pitch.abs() < kept.pitch.abs());
}

#[test]
fn a_head_with_no_driven_axle_counts_as_driven_on_all() {
    let mut sim = VehicleSimulation::new_standard_bus(10_000.0, 5.0);
    sim.axles
        .iter_mut()
        .for_each(|a| a.config.is_driven = false);
    let (driven, braked) = sim.friction_loads_n();
    assert!((driven - braked).abs() < 1.0);
    sim.set_trailers(vec![UnitLoad::with_axles(5_000.0, 2, 2)]);
    assert!((sim.driven_mass_kg() - 15_000.0).abs() < 1.0);
}
