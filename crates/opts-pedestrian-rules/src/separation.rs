//! Walkers behind walkers: a walker does not walk faster than the walker ahead of it when both
//! share a path (the same lane, the same direction, about the same sideways band).
//!
//! The rule slows the follower to the leader's speed. It does not stop anybody and does not
//! overtake; walkers on different lanes at a junction are not compared. The distances are the
//! modelling values of this crate, not OMSI's (❓ in the design).

use glam::Vec3;
use opts_sim_pedestrian::pedestrian::net::{PedestrianNet, Walker};

/// How far ahead (m along the lane) a walker is followed.
pub const FOLLOW_AHEAD_M: f32 = 2.0;
/// Walkers whose paths lie this close sideways (m) share one band and follow each other.
pub const SHARE_BAND_M: f32 = 0.6;

/// The speed each walker may keep after separation (m/s), in the order of `walkers`: its own
/// speed, or the lowest speed of the walkers it follows. Walkers without a pose keep their speed.
#[must_use]
pub fn separated_speeds(walkers: &[Walker], net: &PedestrianNet) -> Vec<f32> {
    let poses: Vec<Option<(Vec3, f32)>> = walkers.iter().map(|w| w.pose(net)).collect();
    walkers
        .iter()
        .enumerate()
        .map(|(i, w)| {
            let Some(me) = poses[i] else {
                return w.speed;
            };
            let mut speed = w.speed;
            for (j, other) in walkers.iter().enumerate() {
                if j != i && is_followed(w, me, other, poses[j]) {
                    speed = speed.min(other.speed);
                }
            }
            speed
        })
        .collect()
}

/// Whether `other` (at `theirs`) is a walker ahead of `w` (at `me`) that `w` follows.
fn is_followed(
    w: &Walker,
    me: (Vec3, f32),
    other: &Walker,
    theirs: Option<(Vec3, f32)>,
) -> bool {
    let Some(theirs) = theirs else {
        return false;
    };
    if other.lane != w.lane || other.dir != w.dir {
        return false;
    }
    let ahead = f32::from(w.dir) * (other.s - w.s);
    if !(0.0..=FOLLOW_AHEAD_M).contains(&ahead) {
        return false;
    }
    // Sideways is measured along the walker's right-hand side (see `Walker::pose`).
    let heading = me.1;
    let right = Vec3::new(heading.cos(), -heading.sin(), 0.0);
    ((theirs.0 - me.0).dot(right)).abs() <= SHARE_BAND_M
}
