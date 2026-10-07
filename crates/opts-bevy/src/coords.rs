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
const SYNC_EPSILON: f32 = 1e-5;

/// Tolerance threshold for angular degree comparisons.
const DEG_EPSILON: f32 = 1e-4;

/// Converts an OMSI position vector (Z-up: X right, Y forward, Z up) into Bevy engine space (Y-up: X right, Y up, -Z forward).
///
/// # Examples
///
/// ```
/// use bevy::math::Vec3;
/// use opts_bevy::omsi_to_bevy_pos;
///
/// let omsi_pos = Vec3::new(0.0, 10.0, 2.0);
/// let bevy_pos = omsi_to_bevy_pos(omsi_pos);
/// assert_eq!(bevy_pos, Vec3::new(0.0, 2.0, -10.0));
/// ```
#[must_use]
#[inline]
pub fn omsi_to_bevy_pos(pos: Vec3) -> Vec3 {
    Vec3::new(pos.x, pos.z, -pos.y)
}

/// Converts a Bevy engine position vector (Y-up: X right, Y up, -Z forward) into OMSI space (Z-up: X right, Y forward, Z up).
///
/// # Examples
///
/// ```
/// use bevy::math::Vec3;
/// use opts_bevy::bevy_to_omsi_pos;
///
/// let bevy_pos = Vec3::new(0.0, 2.0, -10.0);
/// let omsi_pos = bevy_to_omsi_pos(bevy_pos);
/// assert_eq!(omsi_pos, Vec3::new(0.0, 10.0, 2.0));
/// ```
#[must_use]
#[inline]
pub fn bevy_to_omsi_pos(pos: Vec3) -> Vec3 {
    Vec3::new(pos.x, -pos.z, pos.y)
}

/// Converts OMSI Euler rotation angles in degrees (yaw around Z, pitch around X, roll around Y) into a Bevy orientation [`Quat`].
///
/// - Yaw: rotation around OMSI $+Z$ maps to rotation around Bevy $+Y$.
/// - Pitch: rotation around OMSI $+X$ maps to rotation around Bevy $+X$.
/// - Roll: rotation around OMSI $+Y$ maps to rotation around Bevy $-Z$.
///
/// # Arguments
///
/// * `yaw_deg` - Yaw angle in degrees (heading turn).
/// * `pitch_deg` - Pitch angle in degrees (elevation tilt).
/// * `roll_deg` - Roll angle in degrees (lateral banking).
#[must_use]
pub fn omsi_euler_to_bevy_quat(yaw_deg: f32, pitch_deg: f32, roll_deg: f32) -> Quat {
    let yaw_rad = yaw_deg.to_radians();
    let pitch_rad = pitch_deg.to_radians();
    let roll_rad = roll_deg.to_radians();

    // In Bevy, +Y is up, +X is right, and -Z is forward.
    // Intrinsic YXZ rotation: yaw around +Y, pitch around +X, roll around -Z (i.e. -roll around +Z).
    Quat::from_euler(EulerRot::YXZ, yaw_rad, pitch_rad, -roll_rad)
}

/// Extracts OMSI Euler angles in degrees `(yaw, pitch, roll)` from a Bevy orientation [`Quat`].
///
/// # Arguments
///
/// * `quat` - Normalized Bevy orientation quaternion.
///
/// # Returns
///
/// A tuple `(yaw_deg, pitch_deg, roll_deg)` in degrees.
#[must_use]
pub fn bevy_quat_to_omsi_euler(quat: Quat) -> (f32, f32, f32) {
    let (yaw_rad, pitch_rad, roll_neg_rad) = quat.to_euler(EulerRot::YXZ);
    (
        yaw_rad.to_degrees(),
        pitch_rad.to_degrees(),
        (-roll_neg_rad).to_degrees(),
    )
}

/// Converts OMSI position, Euler rotation (degrees), and scale into a native Bevy [`Transform`].
#[must_use]
pub fn omsi_to_bevy_transform(pos: Vec3, rot_deg: Vec3, scale: Vec3) -> Transform {
    Transform {
        translation: omsi_to_bevy_pos(pos),
        rotation: omsi_euler_to_bevy_quat(rot_deg.x, rot_deg.y, rot_deg.z),
        scale: Vec3::new(scale.x, scale.z, scale.y),
    }
}

/// Converts a Bevy [`Transform`] into OMSI translation, Euler rotation (degrees), and scale.
#[must_use]
pub fn bevy_to_omsi_transform(transform: &Transform) -> (Vec3, Vec3, Vec3) {
    let omsi_pos = bevy_to_omsi_pos(transform.translation);
    let (yaw, pitch, roll) = bevy_quat_to_omsi_euler(transform.rotation);
    let omsi_rot = Vec3::new(yaw, pitch, roll);
    let omsi_scale = Vec3::new(transform.scale.x, transform.scale.z, transform.scale.y);
    (omsi_pos, omsi_rot, omsi_scale)
}

/// Component tracking raw coordinates in OMSI entity space (Z-up: X right, Y forward, Z up).
#[derive(Component, Debug, Clone, Copy, PartialEq, Reflect)]
#[reflect(Component)]
pub struct OmsiCoords(pub Vec3);

impl OmsiCoords {
    /// Creates a new `OmsiCoords` component from coordinates in meters.
    #[must_use]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self(Vec3::new(x, y, z))
    }

    /// Converts these coordinates to a Bevy engine position vector.
    #[must_use]
    pub fn to_bevy(&self) -> Vec3 {
        omsi_to_bevy_pos(self.0)
    }

    /// Constructs `OmsiCoords` from a Bevy position vector.
    #[must_use]
    pub fn from_bevy(bevy_pos: Vec3) -> Self {
        Self(bevy_to_omsi_pos(bevy_pos))
    }
}

/// Component representing an entity's complete transform in OMSI coordinate space.
///
/// Contains translation (in meters), rotation (in degrees: yaw, pitch, roll), and scale.
#[derive(Component, Debug, Clone, Copy, PartialEq, Reflect)]
#[reflect(Component)]
pub struct OmsiTransform {
    /// Translation offset in OMSI coordinate space (meters: X right, Y forward, Z up).
    pub translation: Vec3,
    /// Euler rotation angles in degrees: `x` is yaw, `y` is pitch, `z` is roll.
    pub rotation_deg: Vec3,
    /// Scale multipliers along OMSI axes (X lateral, Y longitudinal, Z vertical).
    pub scale: Vec3,
}

impl Default for OmsiTransform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation_deg: Vec3::ZERO,
            scale: Vec3::ONE,
        }
    }
}

impl OmsiTransform {
    /// Creates a new `OmsiTransform` with the given translation, zero rotation, and unit scale.
    #[must_use]
    pub fn from_translation(translation: Vec3) -> Self {
        Self {
            translation,
            ..Default::default()
        }
    }

    /// Creates a new `OmsiTransform` with translation and Euler rotation angles in degrees.
    #[must_use]
    pub fn from_translation_rotation(
        translation: Vec3,
        yaw_deg: f32,
        pitch_deg: f32,
        roll_deg: f32,
    ) -> Self {
        Self {
            translation,
            rotation_deg: Vec3::new(yaw_deg, pitch_deg, roll_deg),
            scale: Vec3::ONE,
        }
    }

    /// Converts this `OmsiTransform` into a native Bevy [`Transform`].
    #[must_use]
    pub fn to_bevy_transform(&self) -> Transform {
        omsi_to_bevy_transform(self.translation, self.rotation_deg, self.scale)
    }

    /// Creates an `OmsiTransform` from a Bevy [`Transform`].
    #[must_use]
    pub fn from_bevy_transform(transform: &Transform) -> Self {
        let (translation, rotation_deg, scale) = bevy_to_omsi_transform(transform);
        Self {
            translation,
            rotation_deg,
            scale,
        }
    }
}

/// One-way synchronization system updating Bevy [`Transform`] when [`OmsiTransform`] changes.
pub fn sync_omsi_to_bevy_transforms(
    mut query: Query<(&OmsiTransform, &mut Transform), Changed<OmsiTransform>>,
) {
    for (omsi_tx, mut bevy_tx) in &mut query {
        let target = omsi_tx.to_bevy_transform();
        if !bevy_tx
            .translation
            .abs_diff_eq(target.translation, SYNC_EPSILON)
            || !bevy_tx.rotation.abs_diff_eq(target.rotation, SYNC_EPSILON)
            || !bevy_tx.scale.abs_diff_eq(target.scale, SYNC_EPSILON)
        {
            *bevy_tx = target;
        }
    }
}

/// One-way synchronization system back-propagating Bevy [`Transform`] changes to [`OmsiTransform`].
pub fn sync_bevy_to_omsi_transforms(
    mut query: Query<(&Transform, &mut OmsiTransform), Changed<Transform>>,
) {
    for (bevy_tx, mut omsi_tx) in &mut query {
        let target = OmsiTransform::from_bevy_transform(bevy_tx);
        if !omsi_tx
            .translation
            .abs_diff_eq(target.translation, SYNC_EPSILON)
            || !omsi_tx
                .rotation_deg
                .abs_diff_eq(target.rotation_deg, DEG_EPSILON)
            || !omsi_tx.scale.abs_diff_eq(target.scale, SYNC_EPSILON)
        {
            *omsi_tx = target;
        }
    }
}

/// Query filter matching entities where either [`OmsiTransform`] or [`Transform`] changed.
pub type SyncTransformFilter = Or<(Changed<OmsiTransform>, Changed<Transform>)>;

/// Query data tuple accessing mutable [`OmsiTransform`] and [`Transform`] components.
pub type SyncTransformData = (Mut<'static, OmsiTransform>, Mut<'static, Transform>);

/// Unified bidirectional synchronization system syncing OMSI and Bevy transforms.
///
/// Prevents race conditions and feedback loops by checking which component was mutated.
/// When both components are mutated concurrently (e.g. initial spawn), [`OmsiTransform`]
/// is treated as authoritative.
pub fn sync_coordinate_transforms(mut query: Query<SyncTransformData, SyncTransformFilter>) {
    for (mut omsi_tx, mut bevy_tx) in &mut query {
        let omsi_changed = omsi_tx.is_changed();
        let bevy_changed = bevy_tx.is_changed();

        if omsi_changed && !bevy_changed {
            let target = omsi_tx.to_bevy_transform();
            if !bevy_tx
                .translation
                .abs_diff_eq(target.translation, SYNC_EPSILON)
                || !bevy_tx.rotation.abs_diff_eq(target.rotation, SYNC_EPSILON)
                || !bevy_tx.scale.abs_diff_eq(target.scale, SYNC_EPSILON)
            {
                *bevy_tx = target;
            }
        } else if bevy_changed && !omsi_changed {
            let target = OmsiTransform::from_bevy_transform(&bevy_tx);
            if !omsi_tx
                .translation
                .abs_diff_eq(target.translation, SYNC_EPSILON)
                || !omsi_tx
                    .rotation_deg
                    .abs_diff_eq(target.rotation_deg, DEG_EPSILON)
                || !omsi_tx.scale.abs_diff_eq(target.scale, SYNC_EPSILON)
            {
                *omsi_tx = target;
            }
        } else if omsi_changed && bevy_changed {
            let target = omsi_tx.to_bevy_transform();
            if !bevy_tx
                .translation
                .abs_diff_eq(target.translation, SYNC_EPSILON)
                || !bevy_tx.rotation.abs_diff_eq(target.rotation, SYNC_EPSILON)
                || !bevy_tx.scale.abs_diff_eq(target.scale, SYNC_EPSILON)
            {
                *bevy_tx = target;
            }
        }
    }
}
