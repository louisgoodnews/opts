//! The player's vehicle on the ground (`follow_ground`) and the ground under every unit of an
//! articulated vehicle (`sample_consist_ground`), from the [`GroundIndex`].

use bevy::prelude::*;
use opts_bevy::VehicleController;

use super::GroundIndex;

/// Session vehicles (active or parked) that follow the ground (rail-bound ones follow their track instead).
type GroundBound = (
    Or<(
        With<crate::session_vehicles::SessionVehicle>,
        With<opts_bevy::PlayerVehicle>,
    )>,
    Without<super::RailPlayer>,
);

/// Distance between the two points the pitch is measured from (m).
const PITCH_BASE_M: f32 = 6.0;

/// What `follow_ground` reads and writes of a player vehicle: its simulation and its transform.
type GroundBody = (
    Entity,
    &'static mut VehicleController,
    &'static mut Transform,
);

/// The largest vertical acceleration the ground following reports (m/s²): a frame-time hiccup
/// must not read as a violent jolt.
const MAX_GROUND_ACC: f32 = 30.0;
/// Gravity for a vehicle that has left the ground (m/s²).
const GRAVITY: f32 = 9.81;
/// A wheel whose ground lies this far below the chassis carries no load (m): the vehicle hangs
/// on the other wheels (a road edge, a kerb) until the ground under all wheels is lower.
const UNSUPPORTED_DROP_M: f32 = 0.6;
use opts_sim::vehicle::ground::{GroundSample, MAX_TILT_RAD, RISE_RATE, TILT_RATE};

/// The player's vertical motion between frames.
#[derive(Default, Clone, Copy)]
pub struct BodyMotion {
    /// Falling speed (m/s, positive down).
    fall: f32,
    pitch: f32,
    roll: f32,
    /// Vertical speed of the elevation in the last frame (m/s, up positive).
    rise: f32,
}


/// The longitudinal positions of the front and rear axle (a short default pitch base without
/// axles) and the widest track (2 m when none is known).
fn axle_extent(axles: &[opts_sim::vehicle::axle::AxleState]) -> (f32, f32, f32) {
    let axles = &controller.sim.axles;
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
        (hi > f32::MIN).then_some(hi).unwrap_or(PITCH_BASE_M * 0.5),
        (lo < f32::MAX).then_some(lo).unwrap_or(-PITCH_BASE_M * 0.5),
        if track > 0.5 { track } else { 2.0 },
    )
}

/// The fine unevenness of the road texture under every wheel shakes the body on its springs
/// (M-0146).
fn add_unevenness(
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

/// Puts the player's vehicle on the ground: the height under each of its four wheel positions
/// gives elevation, pitch and roll. Wheels over a drop carry nothing; when all wheels leave the
/// ground the vehicle falls under gravity instead of snapping down.
pub fn follow_ground(
    time: Res<Time>,
    index: Res<GroundIndex>,
    mut motions: Local<std::collections::HashMap<Entity, BodyMotion>>,
    mut vehicles: Query<GroundBody, GroundBound>,
) {
    let dt = time.delta_secs().min(0.1);
    motions.retain(|e, _| vehicles.contains(*e));
    for (entity, mut controller, mut tf) in &mut vehicles {
        let motion = motions.entry(entity).or_default();
        let (front, rear, track) = axle_extent(&controller.sim.axles);
        let ch = &mut controller.sim.chassis;
        let (px, py) = (ch.position.x, ch.position.y);
        let h = ch.heading_rad;
        let (sin, cos) = h.sin_cos();
        let z = ch.elevation_z;
        // Ground height under the wheel at `lon` along and `side` (±1) across the vehicle.
        let at = |lon: f32, side: f32| {
            let lat = side * track * 0.5;
            index.height_at(px + sin * lon + cos * lat, py + cos * lon - sin * lat, z)
        };
        let (Some(fl), Some(fr), Some(rl), Some(rr)) = (
            at(front, -1.0),
            at(front, 1.0),
            at(rear, -1.0),
            at(rear, 1.0),
        ) else {
            continue;
        };
        let heights = [fl, fr, rl, rr];
        let supported: Vec<f32> = heights
            .iter()
            .copied()
            .filter(|g| *g >= z - UNSUPPORTED_DROP_M)
            .collect();
        let target = if supported.len() >= 2 {
            supported.iter().sum::<f32>() / supported.len() as f32
        } else {
            // Nothing carries the vehicle: it falls towards the highest ground below it.
            heights.iter().copied().fold(f32::MIN, f32::max)
        };
        let teleport = (target - z).abs() > 12.0;
        if teleport {
            // Teleports (respawn, tile change): no motion between the two heights.
            ch.elevation_z = target;
            motion.fall = 0.0;
        } else if target < z - 0.02 {
            motion.fall += GRAVITY * dt;
            ch.elevation_z = (z - motion.fall * dt).max(target);
            if ch.elevation_z <= target {
                motion.fall = 0.0;
            }
        } else {
            motion.fall = 0.0;
            ch.elevation_z = z + (target - z) * (1.0 - (-RISE_RATE * dt).exp());
        }
        // The vertical acceleration of following the ground (kerbs, slopes; M-0146).
        let rise = if teleport || dt <= 0.0 {
            0.0
        } else {
            (ch.elevation_z - z) / dt
        };
        ch.ground_vertical_acc_m_s2 = if teleport || dt <= 0.0 {
            0.0
        } else {
            ((rise - motion.rise) / dt).clamp(-MAX_GROUND_ACC, MAX_GROUND_ACC)
        };
        motion.rise = rise;
        let wheelbase = (front - rear).max(1.0);
        let pitch_goal = (((fl + fr) - (rl + rr)) * 0.5 / wheelbase)
            .atan()
            .clamp(-MAX_TILT_RAD, MAX_TILT_RAD);
        let roll_goal = (((fr + rr) - (fl + rl)) * 0.5 / track)
            .atan()
            .clamp(-MAX_TILT_RAD, MAX_TILT_RAD);
        let blend = 1.0 - (-TILT_RATE * dt).exp();
        motion.pitch += (pitch_goal - motion.pitch) * blend;
        motion.roll += (roll_goal - motion.roll) * blend;
        // The body on its springs (M-0073): heave, pitch and roll of the suspension model, on top
        // of the plane through the wheels (its roll is right side down positive, this one's right
        // side up).
        let (heave, spring_pitch, spring_roll) = controller
            .sim
            .body
            .as_ref()
            .map_or((0.0, 0.0, 0.0), |b| (b.heave_m, b.pitch_rad, -b.roll_rad));
        // What the ground under each wheel adds to the plane through the four (the twist), for
        // the springs: front pair and rear pair of the outer axles.
        let twist = (fl - fr - rl + rr) * 0.25;
        let sim = &mut controller.sim;
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
        add_unevenness(sim, &index, (px, py, sin, cos), z);
        tf.translation.y = sim.chassis.elevation_z + heave;
        tf.rotation = Quat::from_rotation_y(-h)
            * Quat::from_rotation_x(motion.pitch + spring_pitch)
            * Quat::from_rotation_z(motion.roll + spring_roll);
    }
}

/// Hands the simulation the ground under every unit of each articulated vehicle, once per fixed
/// step (design D3): the pitch, roll and height of the chassis (for the joint variables) and of
/// the following units are computed by `VehicleSimulation::apply_ground` with the fixed time
/// step, so the joint variables `articulation_<k>_beta`/`_gamma` do not depend on the frame
/// rate. The chassis' visible body motion stays in [`follow_ground`].
///
/// TEMPORARY (Q-ART-09): the heights come from the game's [`GroundIndex`]; they are replaced by
/// the M-0025 ground query when that is merged.
pub fn sample_consist_ground(
    time: Res<Time>,
    index: Res<GroundIndex>,
    mut vehicles: Query<&mut VehicleController, (With<opts_bevy::ArticulatedChain>, GroundBound)>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    for mut controller in &mut vehicles {
        let sim = &mut controller.sim;
        // The height hint selects between layers (bridges): the fixed-step body of the chassis
        // once it has one, else its own elevation.
        let hint = sim
            .head_body
            .map_or(sim.chassis.elevation_z, |b| b.elevation);
        let samples: Vec<Option<GroundSample>> = sim
            .ground_probes()
            .into_iter()
            .map(|probe| {
                let p = probe.points();
                let h = |i: usize| index.height_at(p[i].x, p[i].y, hint);
                let heights = [h(0)?, h(1)?, h(2)?, h(3)?];
                let sample = GroundSample::from_heights(probe, heights);
                Some(sample)
            })
            .collect();
        sim.apply_ground(&samples, dt);
    }
}
