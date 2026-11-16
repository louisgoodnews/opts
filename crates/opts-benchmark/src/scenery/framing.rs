//! Camera poses and projections for the two views, computed from the expected bounds.
//!
//! Positions here are in the **map frame** (x east, y north, z up, metres). The conversion to
//! Bevy (x east, y up, z south) is written out here on purpose instead of calling the game's
//! helpers, so a transform bug in the game cannot hide in the benchmark
//! (`docs/src/architecture/coordinates.md`).

use bevy::math::{Mat4, Vec2, Vec3, Vec4Swizzles};
use bevy::transform::components::Transform;

/// Image size of every view (square).
pub const IMAGE_SIZE: u32 = 512;

/// Map frame (x east, y north, z up) to Bevy (x east, y up, z south).
#[must_use]
pub fn map_to_bevy(p: Vec3) -> Vec3 {
    Vec3::new(p.x, p.z, -p.y)
}

/// Projection of a view.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Projection {
    /// Perspective with a vertical field of view (radians).
    Perspective {
        /// Vertical field of view in radians.
        fov_y: f32,
        /// Near plane distance.
        near: f32,
        /// Far plane distance.
        far: f32,
    },
    /// Orthographic, `extent` metres wide and high.
    Orthographic {
        /// Side length of the square view volume in metres.
        extent: f32,
        /// Near plane distance.
        near: f32,
        /// Far plane distance.
        far: f32,
    },
}

/// One camera: pose in Bevy space plus projection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    /// Eye position (Bevy).
    pub eye: Vec3,
    /// Look-at target (Bevy).
    pub target: Vec3,
    /// Up direction (Bevy).
    pub up: Vec3,
    /// Projection.
    pub projection: Projection,
}

impl View {
    /// The camera's transform: at `eye`, looking at `target` (Bevy cameras look down −Z).
    #[must_use]
    pub fn transform(&self) -> Transform {
        Transform::from_translation(self.eye).looking_at(self.target, self.up)
    }

    /// World-to-view matrix.
    #[must_use]
    pub fn view_from_world(&self) -> Mat4 {
        self.transform().to_matrix().inverse()
    }

    /// Pixel position of a Bevy-space point (x right, y down, origin top left); `None` behind
    /// the camera.
    #[must_use]
    pub fn project(&self, p: Vec3) -> Option<Vec2> {
        let v = (self.view_from_world() * p.extend(1.0)).xyz();
        let ndc = match self.projection {
            Projection::Perspective { fov_y, .. } => {
                if v.z > -1e-4 {
                    return None;
                }
                let f = 1.0 / (fov_y * 0.5).tan();
                Vec2::new(f * v.x / -v.z, f * v.y / -v.z)
            }
            Projection::Orthographic { extent, .. } => {
                Vec2::new(v.x / (extent * 0.5), v.y / (extent * 0.5))
            }
        };
        let s = IMAGE_SIZE as f32;
        Some(Vec2::new((ndc.x + 1.0) * 0.5 * s, (1.0 - ndc.y) * 0.5 * s))
    }
}

mod bounds;
pub use bounds::Bounds;
mod text;
pub use text::text_views;

/// V1 oblique perspective: 35° to the right of the object's front, 25° above, 40° vertical
/// field of view, the bounding sphere filling 80 % of the image height.
#[must_use]
pub fn oblique_view(bounds: &Bounds, heading_deg: f32) -> View {
    view_from(bounds, heading_deg + 35.0, 25.0)
}

/// The camera looks at the bounds from `azimuth_deg` (clockwise from north) and
/// `elevation_deg` above the horizon, framed like V1.
#[must_use]
pub fn view_from(bounds: &Bounds, azimuth_deg: f32, elevation_deg: f32) -> View {
    let (azimuth, elevation) = (azimuth_deg.to_radians(), elevation_deg.to_radians());
    let fov_y = 40f32.to_radians();
    let r = bounds.radius();
    // Angular radius of the sphere = 80 % of the half field of view (in tangent space).
    let half = (0.8 * (fov_y * 0.5).tan()).atan();
    let dist = r / half.sin();
    let dir = Vec3::new(
        elevation.cos() * azimuth.sin(),
        elevation.cos() * azimuth.cos(),
        elevation.sin(),
    );
    let c = bounds.centre();
    View {
        eye: map_to_bevy(c + dir * dist),
        target: map_to_bevy(c),
        up: Vec3::Y,
        projection: Projection::Perspective {
            fov_y,
            near: ((dist - r) * 0.5).max(0.01),
            far: dist + r * 2.0 + 1.0,
        },
    }
}

/// V2 top-down orthographic: north up, east right, horizontal bounds plus 10 %.
#[must_use]
pub fn top_view(bounds: &Bounds) -> View {
    let size = bounds.max - bounds.min;
    let extent = (size.x.max(size.y) * 1.1).max(0.5);
    let c = bounds.centre();
    let eye = Vec3::new(c.x, c.y, bounds.max.z + 10.0);
    View {
        eye: map_to_bevy(eye),
        target: map_to_bevy(Vec3::new(c.x, c.y, bounds.min.z)),
        up: map_to_bevy(Vec3::Y),
        projection: Projection::Orthographic {
            extent,
            near: 0.0,
            far: size.z + 20.0,
        },
    }
}

#[cfg(test)]
mod tests;
