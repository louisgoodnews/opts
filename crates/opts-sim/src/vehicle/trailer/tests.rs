//! Unit tests for `super` (`trailer.rs`).

use super::*;

mod closed_form;
mod curves;

/// A leader whose origin sits at the rear axle, hitch 1.5 m behind it; segment front coupling
/// 1 m ahead of the segment origin, rear axle 8 m behind the hitch.
fn joint(leader_pos: DVec2, heading: f64) -> TrailerKinematics {
    TrailerKinematics::new(
        DVec2::new(0.0, -1.5),
        DVec2::new(0.0, 1.0),
        8.0,
        52.5,
        leader_pos,
        heading,
    )
}

/// Drives the leader along a circle of `radius` (positive = clockwise) at `speed`.
fn drive_circle(
    t: &mut TrailerKinematics,
    radius: f64,
    speed: f64,
    seconds: f64,
) -> Vec<TrailerState> {
    let dt = 0.02;
    let omega = speed / radius;
    let centre = DVec2::new(radius, 0.0); // leader starts at the origin heading north
    let mut out = Vec::new();
    let mut time = 0.0;
    while time < seconds {
        time += dt;
        let h = omega * time;
        // Position on the circle around `centre`, heading tangent (clockwise for radius > 0).
        let pos = centre + DVec2::new(-radius * h.cos(), radius * h.sin());
        out.push(t.step(pos, h, dt));
    }
    out
}
