//! Conversions between OMSI and Bevy axes: positions, angles and whole transforms.

use super::*;


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
