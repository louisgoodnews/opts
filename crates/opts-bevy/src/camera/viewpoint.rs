//! The viewpoint editor of the driver's view (key 2): the eye position and field of view of the
//! cockpit camera, adjustable with live preview (M-0084).
//!
//! A [`Viewpoint`] is an **offset** from the vehicle's own `[add_camera_driver]` preset, so the
//! default (all zero, no FOV) is exactly the vehicle-defined view. [`ViewpointState`] holds what
//! the camera shows (`active`), what is stored (`committed`) and the editor's state; the keys live
//! in [`handle_viewpoint_keys`] (Shift + Tab opens, Tab closes and saves, arrows adjust, Enter saves, Esc cancels,
//! Home resets, M switches between this vehicle and all of its model). Persistence is the
//! application's job: it loads into `committed`/`active` when the
//! vehicle or the sub camera changes and drains [`ViewpointState::request`] to write the file
//! (`opts-game`, `viewpoints.rs`).
//!
//! Axes are the driver's: **X** to the right, **Y** up, **Z** forward, metres.

use bevy::input::ButtonInput;
use bevy::prelude::*;
use bevy::time::Real;
use serde::{Deserialize, Serialize};

use crate::camera::state::{CameraAvailability, CameraMode, CameraState};

/// Largest offset on each axis in metres (the eye stays inside the cab).
pub const OFFSET_LIMIT_M: f32 = 0.6;
/// Smallest and largest horizontal field of view in degrees.
pub const FOV_RANGE_DEG: (f32, f32) = (30.0, 120.0);
/// One step of an offset in metres (1 cm).
pub const OFFSET_STEP_M: f32 = 0.01;
/// One step of the field of view in degrees.
pub const FOV_STEP_DEG: f32 = 1.0;
/// The coarse multiplier while Shift is held.
pub const COARSE_FACTOR: f32 = 10.0;

/// A cockpit viewpoint: the offset from the vehicle-defined eye position and an optional FOV.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Viewpoint {
    /// Lateral offset in metres, positive to the right.
    #[serde(default)]
    pub x: f32,
    /// Height offset in metres, positive up.
    #[serde(default)]
    pub y: f32,
    /// Longitudinal offset in metres, positive forward.
    #[serde(default)]
    pub z: f32,
    /// Horizontal field of view in degrees; `None` keeps the vehicle's own.
    #[serde(default)]
    pub fov_deg: Option<f32>,
}

impl Viewpoint {
    /// Whether this is the vehicle-defined view (nothing to store).
    #[must_use]
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// Whether every value is finite and inside its limit.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        [self.x, self.y, self.z]
            .iter()
            .all(|v| v.is_finite() && v.abs() <= OFFSET_LIMIT_M + 1e-4)
            && self
                .fov_deg
                .is_none_or(|f| f.is_finite() && (FOV_RANGE_DEG.0..=FOV_RANGE_DEG.1).contains(&f))
    }

    /// The same viewpoint with every value brought inside its limit (non-finite values become 0
    /// or the vehicle's FOV) and rounded to whole centimetres and tenths of a degree.
    #[must_use]
    pub fn clamped(self) -> Self {
        let off = |v: f32| {
            if v.is_finite() {
                ((v.clamp(-OFFSET_LIMIT_M, OFFSET_LIMIT_M)) * 100.0).round() / 100.0
            } else {
                0.0
            }
        };
        Self {
            x: off(self.x),
            y: off(self.y),
            z: off(self.z),
            fov_deg: self
                .fov_deg
                .filter(|f| f.is_finite())
                .map(|f| (f.clamp(FOV_RANGE_DEG.0, FOV_RANGE_DEG.1) * 10.0).round() / 10.0),
        }
    }

    /// The offset in the vehicle's Bevy frame (right = +X, up = +Y, forward = -Z).
    #[must_use]
    pub fn local_offset(&self) -> Vec3 {
        Vec3::new(self.x, self.y, -self.z)
    }
}

/// A row of the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewpointRow {
    /// Lateral offset.
    #[default]
    X,
    /// Height.
    Y,
    /// Longitudinal offset.
    Z,
    /// Field of view.
    Fov,
}

impl ViewpointRow {
    /// All rows in display order.
    pub const ALL: [Self; 4] = [Self::X, Self::Y, Self::Z, Self::Fov];

    /// A stable id (protocol, UI ids).
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::X => "x",
            Self::Y => "y",
            Self::Z => "z",
            Self::Fov => "fov",
        }
    }

    /// The row after this one (wraps).
    #[must_use]
    pub fn next(self) -> Self {
        Self::ALL[(self as usize + 1) % Self::ALL.len()]
    }

    /// The row before this one (wraps).
    #[must_use]
    pub fn prev(self) -> Self {
        Self::ALL[(self as usize + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

/// What the application has to do with the stored viewpoints.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewpointRequest {
    /// Store this viewpoint for the current vehicle and camera (the default removes the entry,
    /// or keeps an explicit default when the vehicle's model has a viewpoint).
    Save(Viewpoint),
    /// Store this viewpoint for every vehicle of the current model (and drop the current
    /// vehicle's own override).
    SaveModel(Viewpoint),
}

mod keys;
mod state;

pub use keys::*;
pub use state::*;

#[cfg(test)]
mod tests;
