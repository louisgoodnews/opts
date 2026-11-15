//! Where the particles are: fixed seeds, the wrapped particle position and the quad corners.

use bevy::prelude::*;
use opts_bevy::weather::PrecipPlan;
use opts_core::formats::weather::PrecipKind;

use super::{BOX_XZ, BOX_Y};

pub(super) const BELOW_CAMERA: f32 = 8.0;

fn hash(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^ (x >> 16)
}

/// The fixed seed `0..1` of particle `i`.
#[must_use]
pub fn seed_of(i: u32) -> [f32; 3] {
    let f = |salt: u32| {
        (hash(i.wrapping_mul(3).wrapping_add(salt).wrapping_add(0x9e37)) >> 8) as f32 / 16_777_216.0
    };
    [f(0), f(1), f(2)]
}

/// Where a particle is at time `t` (seconds), for a camera at `camera`.
///
/// The particle falls with the plan's fall speed and drifts with its horizontal velocity; its
/// place is wrapped into the box around the camera. Snow flutters by a swing of 0.35 m.
#[must_use]
pub fn particle_position(seed: [f32; 3], t: f64, plan: &PrecipPlan, camera: Vec3) -> Vec3 {
    let wrap = |raw: f64, low: f64, len: f64| low + (raw - low).rem_euclid(len);
    let (lx, ly) = (f64::from(BOX_XZ), f64::from(BOX_Y));
    let x = wrap(
        f64::from(seed[0]) * lx + f64::from(plan.drift.x) * t,
        f64::from(camera.x) - lx / 2.0,
        lx,
    );
    let z = wrap(
        f64::from(seed[2]) * lx + f64::from(plan.drift.y) * t,
        f64::from(camera.z) - lx / 2.0,
        lx,
    );
    let y = wrap(
        f64::from(seed[1]) * ly - f64::from(plan.fall_speed) * t,
        f64::from(camera.y) - f64::from(BELOW_CAMERA),
        ly,
    );
    let mut p = Vec3::new(x as f32, y as f32, z as f32);
    if plan.kind == PrecipKind::Snow {
        let phase = f64::from(seed[0]) * std::f64::consts::TAU * 7.0;
        p.x += ((t * 0.8 + phase).sin() * 0.35) as f32;
        p.z += ((t * 0.7 + phase * 1.3).cos() * 0.35) as f32;
    }
    p
}

/// The four corners of a rain streak at `p`: a strip of the plan's size along the fall direction,
/// turned to face `camera`. Order: start left, start right, end right, end left.
#[must_use]
pub fn rain_corners(p: Vec3, plan: &PrecipPlan, camera: Vec3, fallback_right: Vec3) -> [Vec3; 4] {
    let axis = Vec3::new(plan.drift.x, -plan.fall_speed, plan.drift.y).normalize_or_zero();
    let side = axis
        .cross(p - camera)
        .try_normalize()
        .unwrap_or(fallback_right)
        * (plan.size.x * 0.5);
    let half = axis * (plan.size.y * 0.5);
    [
        p - half - side,
        p - half + side,
        p + half + side,
        p + half - side,
    ]
}

/// The four corners of a snow flake at `p`, facing the camera.
#[must_use]
pub fn flake_corners(p: Vec3, plan: &PrecipPlan, right: Vec3, up: Vec3) -> [Vec3; 4] {
    let (r, u) = (right * (plan.size.x * 0.5), up * (plan.size.y * 0.5));
    [p - r - u, p + r - u, p + r + u, p - r + u]
}
