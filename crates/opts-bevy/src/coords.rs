//! Coordinate conversion bridges between OMSI and Bevy coordinate frames.
//!
//! OMSI uses a **right-handed Z-up** frame for vehicle and world entities:
//! - `+X`: Right (lateral)
//! - `+Y`: Forward (longitudinal)
//! - `+Z`: Up (vertical elevation)
//!
//! Bevy uses a **right-handed Y-up** frame:
//! - `+X`: Right
//! - `+Y`: Up
//! - `-Z`: Forward
//! - `+Z`: Backwards (towards camera)
//!
//! # Transformations
//!
//! Vector translation:
//! - $x_{\text{bevy}} = x_{\text{omsi}}$
//! - $y_{\text{bevy}} = z_{\text{omsi}}$
//! - $z_{\text{bevy}} = -y_{\text{omsi}}$
//!
//! Inverse vector translation:
//! - $x_{\text{omsi}} = x_{\text{bevy}}$
//! - $y_{\text{omsi}} = -z_{\text{bevy}}$
//! - $z_{\text{omsi}} = y_{\text{bevy}}$
//!
//! Rotations:
//! - OMSI Yaw ($\psi$, turning around $+Z$) maps to Bevy rotation around $+Y$.
//! - OMSI Pitch ($\theta$, nose up/down around $+X$) maps to Bevy rotation around $+X$.
//! - OMSI Roll ($\phi$, cant around $+Y$) maps to Bevy rotation around $-Z$.

use bevy::ecs::change_detection::DetectChanges;
use bevy::ecs::component::Component;
use bevy::ecs::query::{Changed, Or};
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::system::Query;
use bevy::ecs::world::Mut;
use bevy::math::{EulerRot, Quat, Vec3};
use bevy::reflect::Reflect;
use bevy::transform::components::Transform;

/// Tolerance threshold for coordinate synchronization to avoid floating-point feedback loops.
pub(super) const SYNC_EPSILON: f32 = 1e-5;

/// Tolerance threshold for angular degree comparisons.
pub(super) const DEG_EPSILON: f32 = 1e-4;
mod components;
mod convert;
mod sync;

pub use components::*;
pub use convert::*;
pub use sync::*;

/// Converts an OMSI vehicle camera preset into a native Bevy [`Transform`].
///
/// In OMSI camera definitions (`[add_camera_driver]`, `[add_camera_pax]`):
/// - Lateral offset $x$ is the vehicle's own lateral coordinate: positive to the right, negative
///   to the left (the Citybus 530's driver presets have $x = -0.74$ and its `[drivpos]` seat
///   $x = -0.733$; the front passenger preset `[add_camera_pax]` $x = +0.8$). An earlier version of
///   this function mirrored $x$ and put the driver's eye on the passenger side.
/// - Longitudinal offset $y$ has positive pointing forward ($y_{\text{omsi}} = y$).
/// - Vertical offset $z$ has positive pointing up ($z_{\text{omsi}} = z$).
/// - `yaw_deg`: 0° forward, -120° left, +120° right.
/// - `pitch_deg`: negative down, positive up.
/// - `dist`: pivot distance offset along viewing direction.
#[must_use]
pub fn camera_preset_to_bevy_transform(
    camera_pos: Vec3,
    dist: f32,
    yaw_deg: f32,
    pitch_deg: f32,
) -> Transform {
    // The preset is in the vehicle's frame like every other position of its files.
    let mut bevy_pos = omsi_to_bevy_pos(camera_pos);

    // Bevy rotation: Yaw around +Y (-yaw because right is negative in Bevy angles), pitch around +X
    let rot = Quat::from_euler(
        EulerRot::YXZ,
        (-yaw_deg).to_radians(),
        pitch_deg.to_radians(),
        0.0,
    );

    // If dist is non-zero, translate along forward line-of-sight (-Z in Bevy space)
    if dist.abs() > 1e-4 {
        let forward = rot * Vec3::NEG_Z;
        bevy_pos += forward * dist;
    }

    Transform {
        translation: bevy_pos,
        rotation: rot,
        scale: Vec3::ONE,
    }
}
