//! The trailer's origin and tilt under pitch and roll: the front coupling lands on the hitch,
//! the axles on the ground the simulation samples, and flat ground and the head do not change.

use super::*;
use glam::{DQuat, DVec3};

/// Pitch 3°, roll 2° (tangents of the plane `z = sy·y + sx·x`).
const SY: f32 = 0.052_407_78;
const SX: f32 = 0.034_920_77;
/// Height of the rear coupling of the head and of the front coupling of the trailer (m).
const HITCH_Z: f64 = 0.6;
const FRONT_Z: f64 = 0.6;

/// Samples the plane `z = sy·y + sx·x` under every probe (the ground of a slope).
fn plane_samples(sim: &VehicleSimulation, sx: f32, sy: f32) -> Vec<Option<GroundSample>> {
    sim.ground_probes()
        .into_iter()
        .map(|probe| {
            let h = probe.points().map(|p| p.y * sy + p.x * sx);
            Some(GroundSample::from_heights(probe, h))
        })
        .collect()
}

/// Runs the ground following to its steady state on a fixed plane (no motion of the vehicle).
fn settle(sim: &mut VehicleSimulation, sx: f32, sy: f32) {
    for _ in 0..800 {
        let s = plane_samples(sim, sx, sy);
        sim.apply_ground(&s, 1.0 / 64.0);
    }
}

/// The rotation of a unit with compass heading `h`, pitch `p` (nose up) and roll `r` (right side up),
/// in OMSI: `Rz(-h) · Rx(p) · Ry(-r)`.
fn pose_rotation(h: f64, p: f64, r: f64) -> DQuat {
    DQuat::from_rotation_z(-h) * DQuat::from_rotation_x(p) * DQuat::from_rotation_y(-r)
}

/// World position of the head's rear coupling (the head turns about its origin).
fn hitch_world(sim: &VehicleSimulation) -> DVec3 {
    let head = sim
        .head_body
        .expect("the head has a body after the ground samples");
    let pos = DVec3::new(
        f64::from(sim.chassis.position.x),
        f64::from(sim.chassis.position.y),
        f64::from(head.elevation),
    );
    let h = f64::from(sim.chassis.heading_rad);
    let rot = pose_rotation(h, f64::from(head.pitch), f64::from(head.roll));
    let local = DVec3::new(
        sim.consist[0].hitch_local.x,
        sim.consist[0].hitch_local.y,
        HITCH_Z,
    );
    pos + rot * local
}

/// World position of a point `m` in trailer `k`'s frame, with the trailer pivoting about its front
/// coupling `f` as the render does: `O + Lv·f + Lv·Rx·Ry·(m - f)`.
fn trailer_point(sim: &VehicleSimulation, k: usize, m: DVec3) -> DVec3 {
    let unit = &sim.consist[k];
    let body = sim.consist_body(k).expect("the trailer has a body");
    let origin = DVec3::new(unit.origin.x, unit.origin.y, f64::from(body.elevation));
    let f = front3(sim, k);
    let level = DQuat::from_rotation_z(-unit.heading);
    let tilt = pose_rotation(0.0, f64::from(body.pitch), f64::from(body.roll));
    origin + level * f + level * tilt * (m - f)
}

fn front3(sim: &VehicleSimulation, k: usize) -> DVec3 {
    let f = sim.consist[k].front_local;
    DVec3::new(f.x, f.y, FRONT_Z)
}

#[test]
fn trailer_front_coupling_lands_on_the_hitch_under_three_degrees_of_pitch_and_two_of_roll() {
    let mut sim = articulated();
    sim.set_consist_couplings_z(0, HITCH_Z, FRONT_Z);
    settle(&mut sim, SX, SY);
    let gap = (trailer_point(&sim, 0, front3(&sim, 0)) - hitch_world(&sim)).length();
    assert!(gap < 1e-3, "coupling gap {gap} m");
}

#[test]
fn trailer_axle_contacts_sit_on_the_ground_the_simulation_samples() {
    let mut sim = articulated();
    sim.set_consist_couplings_z(0, HITCH_Z, FRONT_Z);
    settle(&mut sim, SX, SY);
    let probe = sim.ground_probes()[1];
    let half = f64::from(probe.track_m) * 0.5;
    let rear = f64::from(probe.rear_y);
    for (side, x) in [(0, -half), (1, half)] {
        let sampled = f64::from(probe.points()[side].y * SY + probe.points()[side].x * SX);
        let contact = trailer_point(&sim, 0, DVec3::new(x, rear, 0.0)).z;
        assert!(
            (contact - sampled).abs() < 1e-2,
            "side {side}: contact {contact} vs ground {sampled}"
        );
    }
}

#[test]
fn flat_ground_gives_the_same_trailer_as_before_the_coupling_solve() {
    let mut sim = articulated();
    settle(&mut sim, 0.0, 0.0);
    let body = sim.consist_body(0).unwrap();
    assert!(body.elevation.abs() < 1e-6, "{}", body.elevation);
    assert!(body.pitch.abs() < 1e-6 && body.roll.abs() < 1e-6);
    let mut coupled = articulated();
    coupled.set_consist_couplings_z(0, HITCH_Z, HITCH_Z);
    settle(&mut coupled, 0.0, 0.0);
    let level = coupled.consist_body(0).unwrap();
    assert!(level.elevation.abs() < 1e-6, "{}", level.elevation);
    assert!(level.pitch.abs() < 1e-6 && level.roll.abs() < 1e-6);
}

#[test]
fn head_ground_state_is_the_head_goal_on_a_slope() {
    let mut sim = articulated();
    sim.set_consist_couplings_z(0, HITCH_Z, FRONT_Z);
    settle(&mut sim, SX, SY);
    let goal = plane_samples(&sim, SX, SY)[0].unwrap().goal();
    let head = sim.head_body.unwrap();
    assert!((head.elevation - goal.elevation).abs() < 1e-6);
    assert!((head.pitch - goal.pitch).abs() < 1e-6 && (head.roll - goal.roll).abs() < 1e-6);
}

/// The Kajosoft Citybus 530 articulated pair (`23a_o530g_e2_3` with `23a_o530g_e2_3_trailer`): the
/// head's rear coupling at `(0, -4.421, 0.4)`, the trailer's front coupling at `(0, 4.116, 0.4)`, its
/// rear axle at `y = -0.016` (distance 4.132 m) and its 2.4 m track. Equal coupling heights, so on
/// flat ground the trailer stays level at the ground height.
#[test]
fn kajosoft_articulated_pair_stays_level_on_flat_ground() {
    let mut sim = VehicleSimulation::new_standard_bus(11_500.0, 5.85);
    sim.attach_trailer(
        DVec2::new(0.0, -4.421),
        DVec2::new(0.0, 4.116),
        4.132,
        52.5,
        None,
    );
    sim.set_consist_couplings_z(0, 0.4, 0.4);
    settle(&mut sim, 0.0, 0.0);
    let body = sim.consist_body(0).unwrap();
    assert!(body.elevation.abs() < 1e-6, "{}", body.elevation);
    assert!(body.pitch.abs() < 1e-6 && body.roll.abs() < 1e-6);
}
