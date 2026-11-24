//! Lights of a vehicle model: the interior light and the spotlight.

use glam::Vec3;

/// Interior light source (`[interiorlight]`).
#[derive(Debug, Clone, PartialEq)]
pub struct InteriorLight {
    /// Script variable controlling illumination state.
    pub variable: String,
    /// Effective illumination range in meters.
    pub range: f32,
    /// Color in RGB (0-255).
    pub color_rgb: [u8; 3],
    /// 3D position vector in vehicle space.
    pub pos: Vec3,
}

/// Focused exterior spotlight / headlight (`[spotlight]`).
#[derive(Debug, Clone, PartialEq)]
pub struct Spotlight {
    /// Position of the spotlight emitter in local space.
    pub pos: Vec3,
    /// Direction vector of the beam.
    pub dir: Vec3,
    /// RGB color (0-255).
    pub color_rgb: [u8; 3],
    /// Maximum projection range in meters.
    pub range: f32,
    /// Inner cone angle in degrees.
    pub inner_cone: f32,
    /// Outer cone angle in degrees.
    pub outer_cone: f32,
}
