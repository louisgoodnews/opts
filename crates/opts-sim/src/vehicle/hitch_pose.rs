//! The rigid trailer under its coupling: origin height, pitch and roll from the hitch and the
//! ground under its axles (decided by the project owner).
//!
//! Frame: OMSI (Z up, Y forward, X right). A unit with pitch `p` (nose up) and roll `r` (right side
//! up) carries a point `v` of its frame to `Rx(p)·Ry(-r)·v` about its origin; the compass yaw is
//! about `Z` and changes no height. A following unit pivots about its own front coupling `f`, as the
//! render does (`opts-bevy` `segment_placement`), so its coupling stays at `origin + f` and its
//! wheel contacts at `origin + f + R·(m − f)` for a contact point `m`. The leader's rear coupling `b`
//! sits at `leader.origin + R_leader·b`. Requiring the coupling on the hitch and the axle contacts on
//! the sampled ground fixes the origin height and the two tilts: a pitched trailer's axles are
//! `L·sin p` off the ground the coupling would give, so the tilt cannot come from the ground slope
//! alone.

use glam::{DVec2, DVec3};

use super::ground::{UnitBody, MAX_TILT_RAD};
use super::trailer::{fwd, right};

/// Height `z` above its origin of the point `local` of a unit standing as `body`.
#[must_use]
pub fn height_of(body: &UnitBody, local: DVec3) -> f64 {
    let (p, r) = (f64::from(body.pitch), f64::from(body.roll));
    local.y * p.sin() + (local.x * r.sin() + local.z * r.cos()) * p.cos()
}

/// Height of the leader's rear coupling `hitch` (its own frame) in the world.
#[must_use]
pub fn hitch_height(leader: &UnitBody, hitch: DVec3) -> f64 {
    f64::from(leader.elevation) + height_of(leader, hitch)
}

/// Plan shift (world m) of the rear coupling `hitch` of a leader heading `heading` standing as `lead`: its tilted position minus its flat one, the vector [`super::trailer::TrailerKinematics`]
/// adds to the hitch.
#[must_use]
pub fn hitch_plan_shift(heading: f64, lead: &UnitBody, hitch: DVec3) -> DVec2 {
    let (p, r) = (f64::from(lead.pitch), f64::from(lead.roll));
    // `Rx(p)·Ry(-r)·hitch`, the plan components (x, y) of the tilted coupling.
    let x = hitch.x * r.cos() - hitch.z * r.sin();
    let y = hitch.y * p.cos() - (hitch.x * r.sin() + hitch.z * r.cos()) * p.sin();
    right(heading) * (x - hitch.x) + fwd(heading) * (y - hitch.y)
}

/// The ground heights under the rear axle of a following unit, at its wheel contacts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxleGround {
    /// Position `y` of the axle in the unit's frame (m).
    pub rear_y: f64,
    /// Track width of the axle (m).
    pub track: f64,
    /// Ground height under the left (`-x`) and the right (`+x`) wheel (m).
    pub left: f64,
    pub right: f64,
}

/// Body of a following unit whose front coupling `front` (own frame) sits at `hitch_z` and whose
/// wheel contacts stand on `axle`. The origin lies `front.z` below the coupling. The tilts solve
/// the two contact equations (left and right) for the pitch and roll; both are clamped to
/// [`MAX_TILT_RAD`].
#[must_use]
pub fn trailer_body(hitch_z: f64, front: DVec3, axle: AxleGround) -> UnitBody {
    // Contacts relative to the coupling height: `c = ground − hitch_z`.
    let (cl, cr) = (axle.left - hitch_z, axle.right - hitch_z);
    let (dy, dz) = (axle.rear_y - front.y, -front.z);
    let track = axle.track.max(0.5);
    let (mut p, mut r) = (0.0_f64, 0.0_f64);
    for _ in 0..6 {
        // Right minus left contact: track · sin r · cos p = cr − cl.
        r = ((cr - cl) / (track * p.cos().max(0.1)))
            .clamp(-1.0, 1.0)
            .asin();
        // Their mean: dy · sin p + (−f.x · sin r + dz · cos r) · cos p = (cl + cr) / 2, by Newton.
        let b = -front.x * r.sin() + dz * r.cos();
        let mean = 0.5 * (cl + cr);
        for _ in 0..8 {
            let (s, c) = p.sin_cos();
            let g = dy * s + b * c - mean;
            let dg = dy * c - b * s;
            if dg.abs() < 1e-9 {
                break;
            }
            p -= g / dg;
        }
    }
    UnitBody {
        elevation: (hitch_z - front.z) as f32,
        pitch: p.clamp(-f64::from(MAX_TILT_RAD), f64::from(MAX_TILT_RAD)) as f32,
        roll: r.clamp(-f64::from(MAX_TILT_RAD), f64::from(MAX_TILT_RAD)) as f32,
    }
}
