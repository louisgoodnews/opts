//! The body on its springs under braking, cornering and a bump: pitch, load transfer and roll.

use super::support::*;
use opts_sim::VehicleSimulation;

#[test]
fn braking_pitches_the_body_by_the_centre_of_gravity_and_the_axle_layout() {
    let pitch_in_braking = |cg: f32, half: f32| {
        let mut sim = at_speed(
            VehicleSimulation::from_manifest(&file(10.0, cg, half, 100.0)),
            KMH_50,
        );
        sim.inputs.set_service_brake(1.0);
        for _ in 0..100 {
            sim.step(0.01);
        }
        (
            sim.body.expect("body").pitch_rad,
            sim.axles[0].suspension_deflection_m,
        )
    };
    let (low, _) = pitch_in_braking(0.6, 3.0);
    let (high, front) = pitch_in_braking(1.5, 3.0);
    let (short, _) = pitch_in_braking(1.5, 1.5);
    assert!(
        low < 0.0 && high < low,
        "nose down, more with a high centre of gravity: {low} {high}"
    );
    assert!(
        short < high,
        "and more on a short wheelbase: {short} {high}"
    );
    assert!(
        front > 0.0,
        "the front spring is compressed in braking: {front}"
    );
}

#[test]
fn in_a_steady_brake_the_front_axle_carries_the_load_the_transfer_formula_gives() {
    // 12 t, centre of gravity 1.2 m, wheelbase 6 m, braked at a steady 4 m/s².
    let mut sim = VehicleSimulation::from_manifest(&file(12.0, 1.2, 3.0, 120.0));
    sim.rebuild_body();
    let static_front = sim.body.as_ref().expect("body").axle_load_n(0);
    let body = sim.body.as_mut().expect("body");
    for _ in 0..800 {
        body.step(0.01, -4.0, 0.0, &[]);
    }
    let front = body.axle_load_n(0);
    let want = 12_000.0 * 4.0 * 1.2 / 6.0;
    assert!(
        ((front - static_front) - want).abs() < 0.08 * want,
        "front gains {:.0} N, formula {want:.0} N",
        front - static_front
    );
    assert!(
        (body.total_load_n() - 12_000.0 * 9.81).abs() < 300.0,
        "the loads still carry the weight"
    );
}

#[test]
fn a_turn_rolls_the_body_outwards_and_loads_the_outer_wheels() {
    let mut sim = VehicleSimulation::from_manifest(&file(12.0, 1.2, 3.0, 120.0));
    sim.rebuild_body();
    let body = sim.body.as_mut().expect("body");
    // Turning right at 3 m/s²: the body rolls to the left (right side up, roll negative) and the
    // left wheels carry more.
    for _ in 0..600 {
        body.step(0.01, 0.0, 3.0, &[]);
    }
    assert!(body.roll_rad < 0.0, "roll {}", body.roll_rad);
    let (left, right) = (body.wheels[0].load_n, body.wheels[1].load_n);
    assert!(left > right * 1.1, "left {left:.0} N, right {right:.0} N");
}

#[test]
fn a_bump_under_one_wheel_shifts_load_to_the_diagonal_and_conserves_the_weight() {
    let mut sim = VehicleSimulation::from_manifest(&file(12.0, 1.2, 3.0, 120.0));
    sim.rebuild_body();
    let body = sim.body.as_mut().expect("body");
    let static_loads: Vec<f32> = body.wheels.iter().map(|w| w.static_load_n).collect();
    // The front left wheel stands 4 cm higher: its spring is compressed more.
    for _ in 0..600 {
        body.step(0.01, 0.0, 0.0, &[0.04, 0.0, 0.0, 0.0]);
    }
    let loads: Vec<f32> = body.wheels.iter().map(|w| w.load_n).collect();
    assert!(loads[0] > static_loads[0] * 1.05, "{loads:?}");
    assert!(
        loads[3] > static_loads[3] * 1.02,
        "the diagonal wheel gains: {loads:?}"
    );
    assert!(
        loads[1] < static_loads[1] && loads[2] < static_loads[2],
        "{loads:?}"
    );
    let total: f32 = loads.iter().sum();
    assert!((total - 12_000.0 * 9.81).abs() < 200.0, "{total}");
}
