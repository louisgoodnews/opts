//! The coordinate components: [`OmsiCoords`] and [`OmsiTransform`], with their Bevy conversions.

use super::*;


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
