//! The wheels' ground: the axle layout, the height the vehicle rests at, and the twist and unevenness under the wheels.

use super::*;

/// The longitudinal positions of the front and rear axle (a short default pitch base without
/// axles) and the widest track (2 m when none is known).
pub(super) fn axle_extent(axles: &[opts_sim::vehicle::axle::AxleState]) -> (f32, f32, f32) {
    let track = axles
        .iter()
        .map(|a| a.config.track_width_m)
        .fold(0.0_f32, f32::max);
    let ends = axles
        .iter()
        .map(|a| a.config.longitudinal_m)
        .fold((f32::MIN, f32::MAX), |(hi, lo), l| (hi.max(l), lo.min(l)));
    let (hi, lo) = ends;
    (
        if hi > f32::MIN {
            hi
        } else {
            PITCH_BASE_M * 0.5
        },
        if lo < f32::MAX {
            lo
        } else {
            -PITCH_BASE_M * 0.5
        },
        if track > 0.5 { track } else { 2.0 },
    )
}

/// The fine unevenness of the road texture under every wheel shakes the body on its springs
/// (M-0146).
pub(super) fn add_unevenness(
    sim: &mut opts_sim::vehicle::VehicleSimulation,
    index: &GroundIndex,
    (px, py, sin, cos): (f32, f32, f32, f32),
    z: f32,
) {
    for (a, axle) in sim.axles.iter().enumerate() {
        let lon = axle.config.longitudinal_m;
        let half = axle.config.track_width_m.max(0.5) * 0.5;
        for (k, side) in [-1.0_f32, 1.0].into_iter().enumerate() {
            let lat = side * half;
            let wx = px + sin * lon + cos * lat;
            let wy = py + cos * lon - sin * lat;
            sim.wheel_ground_m[a * 2 + k] += index.unevenness_at(wx, wy, z);
        }
    }
}

/// The height the vehicle rests at: the mean of the wheels that have ground within reach under
/// them; with fewer than two it falls towards the highest ground below it.
pub(super) fn support_height(heights: &[f32; 4], z: f32) -> f32 {
    let supported: Vec<f32> = heights
        .iter()
        .copied()
        .filter(|g| *g >= z - UNSUPPORTED_DROP_M)
        .collect();
    if supported.len() >= 2 {
        supported.iter().sum::<f32>() / supported.len() as f32
    } else {
        // Nothing carries the vehicle: it falls towards the highest ground below it.
        heights.iter().copied().fold(f32::MIN, f32::max)
    }
}

/// What the ground under each wheel adds to the plane through the four (the twist), for the
/// springs: front pair and rear pair of the outer axles.
pub(super) fn set_wheel_twist(sim: &mut opts_sim::vehicle::VehicleSimulation, twist: f32) {
    let wheels = sim.axles.len() * 2;
    sim.wheel_ground_m.clear();
    sim.wheel_ground_m.resize(wheels, 0.0);
    if wheels >= 4 {
        // Axle 0 is the front one by the files' convention; the last is the rear.
        let last = wheels - 2;
        sim.wheel_ground_m[0] = twist;
        sim.wheel_ground_m[1] = -twist;
        sim.wheel_ground_m[last] = -twist;
        sim.wheel_ground_m[last + 1] = twist;
    }
}
