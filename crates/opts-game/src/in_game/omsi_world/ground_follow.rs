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

mod elevation;
mod sample;
mod wheels;
pub use elevation::BodyMotion;
use elevation::{blend_tilt, settle_elevation};
pub use sample::sample_consist_ground;
use wheels::{add_unevenness, axle_extent, set_wheel_twist, support_height};

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
        settle_elevation(ch, motion, (support_height(&heights, z), z), dt);
        blend_tilt(motion, heights, (front - rear, track), dt);
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
        set_wheel_twist(sim, twist);
        add_unevenness(sim, &index, (px, py, sin, cos), z);
        tf.translation.y = sim.chassis.elevation_z + heave;
        tf.rotation = Quat::from_rotation_y(-h)
            * Quat::from_rotation_x(motion.pitch + spring_pitch)
            * Quat::from_rotation_z(motion.roll + spring_roll);
    }
}
