//! Detail by distance: a walker far from the camera keeps its last pose and skips the gait.
//!
//! The design (`docs/src/simulation/pedestrians.md`, LOD) names a billboard or a frozen pose
//! beyond about 60 m; this crate takes the frozen pose, because a billboard needs art that the
//! code does not have. The distance is the modelling value of this crate (❓), and the frame
//! cost of the choice is not measured yet (test phase).

/// Distance from the camera (m) beyond which a walker is drawn frozen.
pub const FULL_DETAIL_M: f32 = 60.0;

/// How a walker is animated this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Detail {
    /// Gait phase advances and the joints are posed every frame.
    Full,
    /// The walker keeps its last pose: no phase advance, no joint update.
    Frozen,
}

/// The detail of a walker at `distance_m` from the camera. A distance that is not a number
/// counts as near, so a missing camera never freezes the crowd.
#[must_use]
pub fn detail_at(distance_m: f32) -> Detail {
    if distance_m > FULL_DETAIL_M {
        Detail::Frozen
    } else {
        Detail::Full
    }
}
