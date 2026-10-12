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

/// Axis-aligned bounds in the map frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    /// Minimum corner.
    pub min: Vec3,
    /// Maximum corner.
    pub max: Vec3,
}

impl Bounds {
    /// Bounds of a point set; `None` when it is empty.
    #[must_use]
    pub fn of(points: impl IntoIterator<Item = Vec3>) -> Option<Self> {
        let mut it = points.into_iter().filter(|p| p.is_finite());
        let first = it.next()?;
        Some(it.fold(
            Self {
                min: first,
                max: first,
            },
            |b, p| Self {
                min: b.min.min(p),
                max: b.max.max(p),
            },
        ))
    }

    /// Centre.
    #[must_use]
    pub fn centre(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// Radius of the enclosing sphere around [`Self::centre`] (at least 0.25 m).
    #[must_use]
    pub fn radius(&self) -> f32 {
        ((self.max - self.min).length() * 0.5).max(0.25)
    }
}

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

/// Two orthographic close-ups of the text-textured triangles, looking at their front and at
/// their back along the area-weighted normal (the winding decides what is "front", and the
/// harness does not know the loader's convention, so it looks from both sides). Things between
/// the eye and the text beyond 1.5 radii are clipped away, so an occluding part cannot hide
/// the text. `None` without triangles.
#[must_use]
pub fn text_views(tris: &[[Vec3; 3]]) -> Option<[View; 2]> {
    let bounds = Bounds::of(tris.iter().flatten().copied())?;
    // Back-to-back duplicates would cancel: align every normal with the largest triangle's.
    let normals: Vec<Vec3> = tris
        .iter()
        .map(|t| (t[1] - t[0]).cross(t[2] - t[0]))
        .collect();
    let reference = normals
        .iter()
        .copied()
        .max_by(|a, b| a.length().total_cmp(&b.length()))
        .unwrap_or(Vec3::Z);
    let mut n = Vec3::ZERO;
    for v in normals {
        n += if v.dot(reference) < 0.0 { -v } else { v };
    }
    let n = if n.length() > 1e-6 {
        n.normalize()
    } else {
        Vec3::Z
    };
    let c = bounds.centre();
    let r = bounds.radius();
    let dist = r * 1.5 + 0.5;
    let up_map = if n.z.abs() > 0.9 { Vec3::Y } else { Vec3::Z };
    let make = |dir: Vec3| View {
        eye: map_to_bevy(c + dir * dist),
        target: map_to_bevy(c),
        up: map_to_bevy(up_map),
        projection: Projection::Orthographic {
            extent: r * 2.2,
            near: 0.0,
            far: dist + r * 1.5 + 0.5,
        },
    };
    Some([make(n), make(-n)])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit_box() -> Bounds {
        Bounds {
            min: Vec3::new(-1.0, -1.0, 0.0),
            max: Vec3::new(1.0, 1.0, 2.0),
        }
    }

    #[test]
    fn top_view_puts_north_up_and_east_right() {
        let v = top_view(&unit_box());
        let centre = v.project(map_to_bevy(Vec3::new(0.0, 0.0, 1.0))).unwrap();
        let east = v.project(map_to_bevy(Vec3::new(1.0, 0.0, 1.0))).unwrap();
        let north = v.project(map_to_bevy(Vec3::new(0.0, 1.0, 1.0))).unwrap();
        assert!((centre - Vec2::splat(256.0)).length() < 1e-3);
        assert!(east.x > centre.x && (east.y - centre.y).abs() < 1e-3);
        assert!(north.y < centre.y && (north.x - centre.x).abs() < 1e-3);
    }

    #[test]
    fn text_views_look_at_both_sides_of_a_wall() {
        let wall = [[
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(4.0, 0.0, 0.0),
            Vec3::new(4.0, 0.0, 2.0),
        ]];
        let [a, b] = text_views(&wall).unwrap();
        assert!((a.eye - b.eye).length() > 1.0);
        let c = map_to_bevy(Vec3::new(2.0, 0.0, 1.0));
        assert!(a.project(c).unwrap().distance(Vec2::splat(256.0)) < 40.0);
        assert!(text_views(&[]).is_none());
        // A back-to-back duplicate must not cancel the normal.
        let mut both = wall.to_vec();
        both.push([wall[0][0], wall[0][2], wall[0][1]]);
        let [c, _] = text_views(&both).unwrap();
        assert!((c.eye - a.eye).length() < 1e-3 || (c.eye - b.eye).length() < 1e-3);
    }

    #[test]
    fn oblique_view_keeps_the_sphere_inside_the_image() {
        let b = unit_box();
        let v = oblique_view(&b, 37.0);
        let c = v.project(map_to_bevy(b.centre())).unwrap();
        assert!((c - Vec2::splat(256.0)).length() < 1e-2);
        for corner in [b.min, b.max, Vec3::new(b.min.x, b.max.y, b.max.z)] {
            let p = v.project(map_to_bevy(corner)).unwrap();
            assert!(p.x > 0.0 && p.x < 512.0 && p.y > 0.0 && p.y < 512.0);
        }
    }
}
