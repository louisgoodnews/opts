//! The segment entity of a following unit: its geometry, joint limits, wheels and fixed-step poses.

use bevy::math::Vec2;
use bevy::prelude::*;

/// Geometry of a following segment, taken from the `.bus` files of the two units.
#[derive(Component, Debug, Clone)]
pub struct ArticulatedSegment {
    /// Rear coupling `(x, y)` of the unit ahead.
    pub hitch_local: Vec2,
    /// Front coupling `(x, y)` of this segment.
    pub front_local: Vec2,
    /// Height `z` of the segment's front coupling above its origin (m, OMSI); the tilt pivots about
    /// the coupling (see [`super::system::segment_placement`]).
    pub front_z: f32,
    /// Height `z` of the rear coupling of the unit ahead above its origin (m, OMSI): the hitch the
    /// simulation's coupling solve puts the front coupling on.
    pub hitch_z: f32,
    /// Position `y` of the segment's rearmost axle in its own frame.
    pub rear_axle_y: f32,
    /// Joint limit on `|α|` in degrees (`[coupling_front_character]`).
    pub max_alpha_deg: f32,
    /// Mass and axles of the unit for the consist dynamics (`None`: the unit is massless).
    pub load: Option<opts_sim::vehicle::consist::UnitLoad>,
    /// Limits of the pitch `β` in degrees `(min, max)` (`[coupling_front_character]`); the
    /// `articulation_<k>_beta` variable is clamped to them. `None`: unlimited.
    pub beta_limits_deg: Option<(f32, f32)>,
    /// The segment hangs backwards (its model is turned by 180°). Its coupling points and rear
    /// axle are then given mirrored (`-x`, `-y`) and the model is rotated half a turn.
    pub reversed: bool,
    /// Pose after the previous and the last fixed step `(x, y, heading)` in simulation space; the
    /// rendered pose interpolates between them.
    pub(super) prev_pose: Option<(Vec2, f32)>,
    pub(super) curr_pose: Option<(Vec2, f32)>,
    /// Height, pitch and roll of the segment on the ground after the previous and the last fixed
    /// step (from the simulation's ground following; `None`: no ground sample yet, the segment
    /// keeps the root's height and stays level). The rendered pose interpolates between them.
    pub(super) prev_ground: Option<SegmentGround>,
    pub(super) curr_ground: Option<SegmentGround>,
    /// Index of the segment's first axle in the consist-wide numbering the model uses
    /// (`Wheel_Rotation_<index>_L`: the head's axles come first, then each trailer's).
    pub first_axle: usize,
    /// The wheels of the segment's own axles (radius, spin), rolling with the segment's travel.
    pub wheels: Vec<SegmentWheel>,
}

/// One axle of a following segment: both wheels roll alike.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SegmentWheel {
    /// Wheel radius (m).
    pub radius: f32,
    /// Accumulated spin (rad), growing while the segment rolls forward along its model.
    pub rotation_rad: f32,
}

/// How a segment lies on the ground (see [`ArticulatedSegment::ground`]).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SegmentGround {
    /// Height of the segment's origin above the datum (m).
    pub elevation: f32,
    /// Pitch of the segment's body, nose up positive (rad).
    pub pitch: f32,
    /// Roll of the segment's body, right side up positive (rad).
    pub roll: f32,
}

impl ArticulatedSegment {
    /// Describes a segment; it hangs straight behind its leader until the first step.
    #[must_use]
    pub fn new(hitch_local: Vec2, front_local: Vec2, rear_axle_y: f32, max_alpha_deg: f32) -> Self {
        Self {
            hitch_local,
            front_local,
            front_z: 0.0,
            hitch_z: 0.0,
            rear_axle_y,
            max_alpha_deg,
            load: None,
            beta_limits_deg: None,
            reversed: false,
            prev_pose: None,
            curr_pose: None,
            prev_ground: None,
            curr_ground: None,
            first_axle: 0,
            wheels: Vec::new(),
        }
    }

    /// `beta` (degrees) limited to the joint's pitch range.
    #[must_use]
    pub fn clamp_beta_deg(&self, beta: f32) -> f32 {
        self.beta_limits_deg
            .map_or(beta, |(lo, hi)| beta.clamp(lo, hi))
    }

    /// Gives the segment its wheels: `first_axle` is the consist-wide index of its first axle and
    /// `radii` the wheel radii of its axles in order.
    pub fn set_wheels(&mut self, first_axle: usize, radii: impl IntoIterator<Item = f32>) {
        self.first_axle = first_axle;
        self.wheels = radii
            .into_iter()
            .map(|radius| SegmentWheel {
                radius: radius.max(0.05),
                rotation_rad: 0.0,
            })
            .collect();
    }

    /// Rolls the wheels by the segment's own travel `from -> to` (simulation space, kinematic
    /// frame, heading `heading` compass rad) and returns the signed distance along the model's
    /// forward direction.
    pub(super) fn roll(&mut self, from: Vec2, to: Vec2, heading: f32) -> f32 {
        let forward = Vec2::new(heading.sin(), heading.cos());
        let mut d = (to - from).dot(forward);
        if self.reversed {
            d = -d;
        }
        for w in &mut self.wheels {
            w.rotation_rad = (w.rotation_rad + d / w.radius).rem_euclid(std::f32::consts::TAU);
        }
        d
    }

    /// How the segment lies on the ground after the last fixed step, if it has been sampled.
    #[must_use]
    pub fn ground(&self) -> Option<SegmentGround> {
        self.curr_ground
    }

    /// Position and heading (compass, rad) of the segment's own frame after the last fixed step
    /// (a reversed segment's frame is turned half a circle against the kinematic one).
    #[must_use]
    pub fn simulated_pose(&self) -> Option<(Vec2, f32)> {
        let (p, h) = self.curr_pose?;
        Some((
            p,
            if self.reversed {
                h + std::f32::consts::PI
            } else {
                h
            },
        ))
    }
}
