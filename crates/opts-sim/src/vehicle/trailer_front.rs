//! Joint transform of an articulated bus's bellow: the trailer-side vertices follow the trailer's
//! rigid motion about the hitch `H`, not a single ring.
//!
//! Frame: OMSI (Z up, Y forward, X right), the leader's frame with the leader at rest.
//!
//! Sign decision (read from the code, not assumed):
//! - Yaw. [`super::trailer::TrailerKinematics::step`] sets `heading = leader + α` with `α`
//!   clockwise (compass). A compass heading `h` is the forward vector `(sin h, cos h)`
//!   (`trailer.rs`, `fwd`), which is a rotation by `-h` about +Z. So a turn by `α` clockwise is
//!   `Rz(-α)`. The origin `hitch - front_offset(heading)` (`trailer.rs`, `reseed`/`step`) is the
//!   same rotation about `H`, checked in the tests.
//! - Pitch. The render applies `Quat::from_rotation_x(pitch)` with `pitch = consist_body(k).pitch`
//!   (`opts-bevy/src/articulation/system.rs`, `articulation_render_system`, and the sync at line
//!   55). The published angle is `β = leader pitch - pitch` (`opts-sim/src/vehicle/ground.rs`,
//!   `joint_beta_deg`, and `consist_ops.rs`, `joint_angles_deg`). So the relative pitch is `-β`.
//! - Roll. The render applies `from_rotation_z(roll)` in Bevy coordinates. The Bevy mapping is
//!   `(x, y, z) -> (x, z, -y)` (`opts-bevy/src/coords/convert.rs`, `omsi_to_bevy_pos`), so Bevy
//!   `Z` is OMSI `-Y` and Bevy `Y` is OMSI `Z`. The published angle is `γ = roll - leader roll`
//!   (`consist_ops.rs`, `joint_angles_deg`), so the relative roll is `+γ` in Bevy, `Ry(-γ)` in OMSI.
//! - Bevy `Y` rotation `-h` is OMSI `Rz(-h)`, the same as the yaw above, so the render's
//!   `from_rotation_y(-h) * Rx(pitch) * Rz(roll)` becomes `Rz(-α) * Rx(-β) * Ry(-γ)` in OMSI.

use glam::{DMat4, DQuat, DVec3};

/// The relative rotation of the trailer against the leader, in OMSI: `Rz(-α) · Rx(-β) · Ry(-γ)`.
///
/// `alpha` is the clockwise yaw of [`super::trailer::TrailerState`] (positive = turned clockwise),
/// not the published `articulation_<k>_alpha` (which is its negation). `beta` and `gamma` are in
/// radians with the sign of [`super::consist_ops`]'s `joint_angles_deg`, converted from degrees.
#[must_use]
pub fn relative_rotation(alpha: f64, beta: f64, gamma: f64) -> DQuat {
    DQuat::from_rotation_z(-alpha) * DQuat::from_rotation_x(-beta) * DQuat::from_rotation_y(-gamma)
}

/// The joint transform `J = T(H) · R_rel · T(-H)`: the trailer's rigid motion about the hitch `H`
/// (leader OMSI frame). `J` maps the rest position of a bellow vertex to where the trailer
/// carries it, and it fixes `H` for any angles.
#[must_use]
pub fn joint_transform(hitch: DVec3, alpha: f64, beta: f64, gamma: f64) -> DMat4 {
    DMat4::from_translation(hitch)
        * DMat4::from_quat(relative_rotation(alpha, beta, gamma))
        * DMat4::from_translation(-hitch)
}

#[cfg(test)]
mod tests;
